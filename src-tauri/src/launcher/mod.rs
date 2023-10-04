mod manifest;
pub mod altarik;

use std::path::{Path, self, PathBuf};

use anyhow::{Result, bail};
use reqwest::{Client, StatusCode};
use serde::{Serialize, Deserialize};
use tokio::{fs::{self, File, OpenOptions}, io::{AsyncWriteExt, AsyncSeekExt}, sync::mpsc};

use crate::authentification::GameProfile;

use self::{manifest::{VersionDetail, get_version_manifest, get_version_from_manifest, get_version_detail, Library, OSName, get_version_assets, AssetsManifest}, altarik::Chapter};


#[cfg(target_os="windows")]
const ACTUAL_OS: OSName = OSName::Windows;
#[cfg(target_os="linux")]
const ACTUAL_OS: OSName = OSName::Linux;
#[cfg(target_os="macos")]
const ACTUAL_OS: OSName = OSName::MacOsX;

#[cfg(not(any(target_arch="x86_64", target_arch="x86")))]
compile_error!("Your architecture is not supported");

#[derive(Clone, serde::Serialize, Debug)]
pub struct ProgressMessage {
    p_type: String,
    current: usize,
    total: usize,
}

pub struct ClientOptions<'a> {
    pub authorization: GameProfile,
    pub log_channel: mpsc::Sender<ProgressMessage>,
    pub root_path: &'a Path,
    pub java_path: &'a Path,
    /// deprecated, will be remove
    pub version_number: String,
    pub version_type: VersionType,
    // version_custom: String, // for a next update
    pub memory_min: String,
    pub memory_max: String,
}

pub struct MinecraftClient<'a> {
    opts: &'a ClientOptions<'a>,
    details: VersionDetail,
    assets: AssetsManifest,
    reqwest_client: Client,

}

impl<'a> MinecraftClient<'_> {
    pub async fn new(opts: &'a ClientOptions<'a>) -> Result<MinecraftClient<'a>> {
        let reqwest_client = Client::new();
        let manifest = Self::load_manifest(&reqwest_client, &opts).await?;
        let details = manifest.0;
        let assets = manifest.1;
        Ok(MinecraftClient {
            opts,
            reqwest_client,
            details,
            assets,
        })
    }

    async fn load_manifest(reqwest_client: &Client, opts: &ClientOptions<'a>) -> Result<(VersionDetail, AssetsManifest)> {
        let manifest = get_version_manifest(&reqwest_client).await?;
        let version = get_version_from_manifest(&manifest, opts.version_number.clone(), &opts.version_type).await?;
        let details = get_version_detail(&reqwest_client, version).await?;
        let version_assets = get_version_assets(reqwest_client, &details.asset_index).await?;
        Ok((details, version_assets))
    }
    
    async fn create_dirs(&self) -> Result<()> {
        let folders = vec![
            self.opts.root_path.join("libraries"),
            self.opts.root_path.join("assets").join("objects"),
            self.opts.root_path.join("assets").join("indexes"),
            self.opts.root_path.join("runtime").join("download"),
            ];
        let mut tasks = Vec::with_capacity(folders.len());
        for folder in folders {
            if !folder.exists() {
                tasks.push(tokio::spawn(async { fs::create_dir_all(folder).await }));
            }
        }
        for task in tasks {
            let _ = task.await?;
        }
        Ok(())
    }

    async fn save_version_index(&self) -> Result<()> {
        let indexes = &self.opts.root_path.join("assets").join("indexes");
        let mut filename = self.details.assets.clone();
        filename.push_str(".json");
        let filepath = indexes.join(filename);
        let file = File::create(filepath).await;
        match file {
            Ok(mut f) => {
                f.write_all(serde_json::to_string(&self.details)?.as_bytes()).await?;
                Ok(())
            },
            Err(err) => bail!(err),
        }
    }

    pub async fn download_requirements(&mut self, chapter: Chapter) -> Result<()> {
        // create root folder if it doesn't exist
        self.create_dirs().await?;
        let lib = &self.opts.root_path.join("libraries");
        let asset = &self.opts.root_path.join("assets").join("objects");
        self.save_version_index().await?;
        self.download_java(chapter).await?;
        self.download_libraries(lib).await?;
        self.download_assets(asset).await?;
        self.opts.log_channel.send(ProgressMessage { p_type: "completed".to_string(), current: 0, total: 0 }).await?;
        Ok(())
    }

    async fn download_java(&mut self, chapter: Chapter) -> Result<()> {
        let download_path = self.opts.root_path.join("runtime").join("download");
        // let extract_path = self.opts.root_path.join("runtime");
        let (url, extension) = match ACTUAL_OS {
            OSName::Linux => {
                (chapter.java.platform.linux, "tar.gz")
            },
            OSName::Windows => {
                (chapter.java.platform.win32, "zip")
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
            .create(true)
            .open(filepath)
            .await?;
            let b = self.reqwest_client.get(url.x64.link.clone()).send().await?.bytes().await?;
            file.write(&b).await?;
            println!("{} downloaded", url.x64.name)
        } else {
            println!("{} already downloaded", url.x64.name)
        }
        Ok(())
    }

    async fn download_libraries(&mut self, lib: &PathBuf) -> Result<()> {
        self.filter_non_necessary_librairies();
        let total = self.details.libraries.len();
        for (progress, i) in self.details.libraries.iter().enumerate() {
            let p = i.downloads.artifact.path.clone();
            let mut splited = p.split("/").collect::<Vec<&str>>();
            let filename = splited.pop().ok_or(anyhow::anyhow!("Invalid filename"))?; // remove last element
            let p = splited.join(path::MAIN_SEPARATOR_STR);
            let p = &lib.join(p);
            fs::create_dir_all(p).await?;
            let file_path = p.join(filename);
            let mut file = Self::select_file_option(&file_path, i.downloads.artifact.size).await?;
            
            let size = file.seek(std::io::SeekFrom::End(0)).await?;
            file.seek(std::io::SeekFrom::Start(0)).await?;
            if size != i.downloads.artifact.size {
                let url = i.downloads.artifact.url.clone();

                let mut sha_url = url.clone();
                sha_url.push_str(".sha1");
                let sha1 = self.reqwest_client
                .get(sha_url)
                .send()
                .await?;
                if sha1.status() == StatusCode::OK {
                    let sha1 = sha1.text().await?;
                    if sha1 != i.downloads.artifact.sha1 {
                        bail!("Sha1 {:?} of {} is invalid, a malicious file might be present on the remote server, should be {}", sha1, i.name, i.downloads.artifact.sha1)
                    }
                }
                
                let content = self.reqwest_client
                .get(url)
                .send()
                .await?
                .bytes()
                .await?;
                file.write_all(&content).await?;
                println!("{} downloaded", i.name);
            } else {
                println!("{} already downloaded", i.name);
            }
            self.opts.log_channel.send( ProgressMessage { p_type: "libraries".to_string(), current: progress + 1, total }).await?;
        }
        Ok(())
    }

    async fn select_file_option(file_path: &PathBuf, expected_size: u64) -> Result<File> {
        if (&file_path).exists() {
            let f = fs::File::open(&file_path).await;
            match f {
                Ok(mut f) => { if f.seek(std::io::SeekFrom::End(0)).await? == expected_size { Ok(f) } else { Ok(fs::File::create(file_path).await?) } },
                Err(err) => bail!(err),
            }
        } else {
            Ok(fs::File::create(file_path).await?)
        }
    }

    /// Filter non necessary librairies for the current OS
    fn filter_non_necessary_librairies(&mut self) {
        self.details.libraries.retain(|e| { Self::should_use_library(e) });  
    }

    async fn download_assets(&mut self, object_folder: &PathBuf) -> Result<()> {
        let total: usize = self.assets.objects.len();
        for (progress, (_, value)) in self.assets.objects.iter().enumerate() {
            let hash = value.hash.clone();
            let two_hex = hash.chars().take(2).collect::<String>();
            let hex_folder = object_folder.join(&two_hex);
            if !hex_folder.exists() {
                fs::create_dir(&hex_folder).await?;
            }
            
            let file_path = hex_folder.join(&hash);
            let mut file = Self::select_file_option(&file_path, value.size).await?;
            let size = file.seek(std::io::SeekFrom::End(0)).await?;
            file.seek(std::io::SeekFrom::Start(0)).await?;

            if size != value.size {
                let url = format!("https://resources.download.minecraft.net/{}/{}", two_hex, hash);
                let received = self.reqwest_client
                    .get(url)
                    .send()
                    .await?
                    .bytes()
                    .await?;
                file.write_all(&received).await?;
                println!("{} downloaded", value.hash);
            } else {
                println!("{} already downloaded", value.hash);
            }
            self.opts.log_channel.send( ProgressMessage { p_type: "assets".to_string(), current: progress + 1, total }).await?;
        }
        Ok(())
    }

    fn should_use_library(library: &Library) -> bool {
        match &library.rules {
            Some(rules) => {
                for i in rules.iter().enumerate() {
                    let op = if i.1.action == "allow" {
                        true
                    } else {
                        false
                    };
                    if i.1.os.name == ACTUAL_OS {
                        return op;
                    }
                }
                false
            },
            None => {
                true
            }
        }
    }

    pub fn launch_game(&self) {
        // TODO
    }
    
}



#[derive(PartialEq, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum VersionType {
    #[serde(alias = "release")]
    Release,
    #[serde(alias = "snapshot")]
    Snapshot,
    #[serde(alias = "old_alpha")]
    OldAlpha,
    #[serde(alias = "old_beta")]
    OldBeta,
}
