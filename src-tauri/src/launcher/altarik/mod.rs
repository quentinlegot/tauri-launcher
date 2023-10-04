use std::path::{PathBuf, Path};

use anyhow::{Result, bail};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use async_zip::base::read::seek::ZipFileReader;
use tokio::{fs::{OpenOptions, File, self}, sync::mpsc,  io::AsyncWriteExt};
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

    pub async fn extract_java(&self, root_path: &Path) -> Result<()> {
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
        // if !extract_path.exists() {
        //     fs::create_dir(extract_path.clone()).await?;
        // }
        extract_zip(archive_path, extract_path).await?;

        Ok(())
    }

}

impl ModsPack {


    pub async fn download_mods() {
        // TODO
    }

    
}

async fn extract_targz(archive_path: PathBuf, extract_path: PathBuf) -> Result<()> {
    let file = File::open(archive_path.clone()).await?;
    let mut archive = Archive::new(file);
    let mut entries = archive.entries()?;
    while let Some(entry) = entries.next().await {
        let file = entry?;
        println!("{:?}", archive_path);
        println!("{}", file.path()?.to_string_lossy().to_string());
    }
    Ok(())
}

async fn extract_zip(archive_path: PathBuf, extract_path: PathBuf) -> Result<()> {
    // TODO add log_channel to send progression to user
    let file = File::open(archive_path.clone()).await?;
    let mut reader = ZipFileReader::with_tokio(file).await?;
    for index in 0..reader.file().entries().len() {
        if let Some(entry) = reader.file().entries().get(index) {
            let entry = entry.entry();
            // println!("{}", entry.filename().as_str()?);
            let filename = entry.filename().as_str()?;
            let path = extract_path.join(filename);
            if entry.dir()? {
                if !path.exists() {
                    fs::create_dir_all(path).await?;
                }
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