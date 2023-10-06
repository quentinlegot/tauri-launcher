use std::path::{PathBuf, Path};

use anyhow::{Result, bail, anyhow};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use async_zip::base::read::seek::ZipFileReader;
use sha1::{Digest, Sha1};
use tokio::{fs::{OpenOptions, File, self}, sync::mpsc,  io::{AsyncWriteExt, AsyncReadExt}};
use tokio_tar::Archive;
use tokio_stream::StreamExt;

use crate::launcher::ProgressMessage;

use super::{ACTUAL_OS, manifest::OSName};


#[derive(Serialize, Deserialize, Clone)]
pub struct AltarikManifest {
    pub chapters: Vec<Chapter>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Chapter {
    pub title: String,
    pub description: String,
    #[serde(rename(serialize = "minecraftVersion", deserialize = "minecraftVersion"))]
    pub minecraft_version: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    pub mc_type: String,
    #[serde(rename(serialize = "customVersion", deserialize = "customVersion"))]
    pub custom_version: String,
    pub modspack: ModsPack,
    pub java: Java,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ModsPack {
    pub mods: Vec<String>,
    pub sha1sum: Vec<String>
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Java {
    pub platform: JavaPlatform,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JavaPlatform {
    pub win32: Option<JavaPlatformArch>,
    pub linux: Option<JavaPlatformArch>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JavaPlatformArch {
    pub x64: JavaDetails
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JavaDetails {
    pub name: String,
    pub link: String,
    pub sha256sum: String,
}

impl AltarikManifest {

    pub async fn get_altarik_manifest(reqwest: &Client) -> Result<AltarikManifest> {
        let received: AltarikManifest = reqwest
        .get("https://launcher.altarik.fr")
        .send()
        .await?
        .json()
        .await?;
        Ok(received)
    }

}

impl Chapter {

    

}

impl JavaPlatform {

    pub async fn download_java(&self, root_path: &Path, reqwest: &Client, log_channel: mpsc::Sender<ProgressMessage>) -> Result<()> {
        let download_path = root_path.join("runtime").join("download");
        let (url, extension) = match ACTUAL_OS {
            OSName::Linux => (self.linux.clone(), "tar.gz"),
            OSName::Windows => (self.win32.clone(), "zip"),
            _ => bail!("Your current is not supported")
        };
        let url = match url {
            Some(url) => url,
            None => bail!("No available executable available for your platform")
        };

        let filepath = download_path.join(format!("{}.{}", url.x64.name.clone(), extension));
        let mut should_download = false;
        if !filepath.exists() {
            should_download = true;
        } else {
            let hash = sha256::try_digest(filepath.clone());
            match hash {
                Ok(hash) => {
                    if hash != url.x64.sha256sum {
                        println!("Hash of java archive is not correct, redownloading");
                        should_download = true;
                    }
                },
                Err(_) => should_download = true
            }
        }
        if should_download {
            println!("Downloading java");
            if filepath.exists() {
                fs::remove_file(filepath.clone()).await?; // remove content before writing and appending to it
            }
            let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(filepath)
            .await?;
            let mut res = reqwest.get(url.x64.link.clone()).send().await?;
            log_channel.send(ProgressMessage { p_type: String::from("java"), current: 0, total: 100 }).await?;
            let length = if let Some(length) = res.content_length() {
                length as usize
            } else {
                0
            };
            let mut downloaded_size = 0;
            while let Some(chunk) = res.chunk().await? {
                downloaded_size += chunk.len();
                file.write_all(&chunk).await?;
                log_channel.send(ProgressMessage { p_type: String::from("java"), current: if length != 0 { downloaded_size / length } else { 0 }, total: 100 }).await?;
            }
            println!("{} downloaded", url.x64.name)
        } else {
            println!("{} already downloaded", url.x64.name)
        }
        Ok(())
    }

    pub async fn extract_java(&self, root_path: &Path, log_channel: mpsc::Sender<ProgressMessage>) -> Result<()> {
        let (url, extension) = match ACTUAL_OS {
            OSName::Linux => {
                (self.linux.clone(), "tar.gz")
            },
            OSName::Windows => {
                (self.win32.clone(), "zip")
            },
            _ => {
                bail!("Your current is not supported")
            }
        };
        let url = match url {
            Some(url) => url,
            None => bail!("No available executable available for your platform")
        };
        let archive_path = root_path.join("runtime").join("download").join(format!("{}.{}", url.x64.name, extension));
        let extract_path = root_path.join("runtime");
        extract_zip(archive_path, extract_path, log_channel).await?;

        Ok(())
    }

}

impl ModsPack {


    pub async fn download_mods(&self, reqwest: &Client, modpack_dir: PathBuf, log_channel: mpsc::Sender<ProgressMessage>) -> Result<()> {
        for index in 0..self.mods.len() {
            log_channel.send(ProgressMessage { p_type: "mods".to_string(), current: index, total: self.mods.len() }).await?;
            let mod_url = self.mods.get(index).ok_or(anyhow!("Cannot get mod download link"))?;
            let sha1 = self.sha1sum.get(index).ok_or(anyhow!("Cannot verify mod integrity"))?;
            let filepath = modpack_dir.join(format!("modpack{}.zip", index));
            let should_download = self.should_download_mod(&sha1, &filepath).await?;
            if should_download {
                println!("Need to download mod {}", mod_url);
                let mut res = reqwest.get(mod_url)
                .send()
                .await?;
                let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .append(true)
                .open(filepath)
                .await?;
                while let Some(chunk) = res.chunk().await? {
                    file.write_all(&chunk).await?;
                }
            }
        }
        Ok(())
    }

    async fn should_download_mod(&self, mod_sha1: &&String, filepath: &PathBuf) -> Result<bool> {
        if filepath.exists() {
            let mut hasher = Sha1::new();
            let mut file = File::open(filepath).await?;
            let mut content = Vec::new();
            file.read(&mut content).await?;
            hasher.update(content);
            let hash = hasher.finalize();
            // let b16 = base16ct::upper::encode_string(hash);
            if &&format!("{:x}", &hash.clone()) != mod_sha1 {
                println!("Correct: {:?}, current: {:X}", mod_sha1, hash);
                fs::remove_file(filepath).await?;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(true)
        }
    }

    
}

async fn extract_targz(archive_path: PathBuf, extract_path: PathBuf, log_channel: mpsc::Sender<ProgressMessage>) -> Result<()> {
    let file = File::open(archive_path.clone()).await?;
    let mut archive = Archive::new(file);
    let mut entries = archive.entries()?;
    while let Some(entry) = entries.next().await {
        let file = entry?;
        println!("{:?}", archive_path);
        println!("{}", file.path()?.to_string_lossy().to_string());
        // TODO when I'll be on a linux
    }
    Ok(())
}

async fn extract_zip(archive_path: PathBuf, extract_path: PathBuf, log_channel: mpsc::Sender<ProgressMessage>) -> Result<()> {
    // TODO add log_channel to send progression to user
    let file = File::open(archive_path.clone()).await?;
    let mut reader = ZipFileReader::with_tokio(file).await?;
    let total = reader.file().entries().len();
    for index in 0..reader.file().entries().len() {
        if let Some(entry) = reader.file().entries().get(index) {
            let entry = entry.entry();
            let filename = entry.filename().as_str()?;
            let path = extract_path.join(filename);
            log_channel.send(ProgressMessage { p_type: "extract".to_string(), current: index + 1, total: total }).await?;
            if entry.dir()? {
                if path.exists() {
                    fs::remove_dir_all(path.clone()).await?; // clear folder before continue, avoid injection
                }
                fs::create_dir_all(path).await?; // recreating the folder then
            } else {
                let mut entry_reader = reader.reader_with_entry(index).await?;
                if path.exists() {
                    fs::remove_file(&path).await?;
                }
                let mut writer = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create_new(true)
                    .open(&path)
                    .await?;
                let mut buf = Vec::with_capacity(1024 * 128);
                entry_reader.read_to_end_checked(&mut buf).await?;
                writer.write_all(&buf).await?;
            }
        }
    }

    Ok(())
}