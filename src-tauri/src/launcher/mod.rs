mod manifest;

use std::path::{Path, self, PathBuf};

use anyhow::{Result, bail};
use reqwest::{Client, StatusCode};
use serde::{Serialize, Deserialize};
use tokio::{fs, io::{AsyncWriteExt, AsyncSeekExt}, sync::mpsc};

use crate::authentification::GameProfile;

use self::manifest::{VersionDetail, get_version_manifest, get_version_from_manifest, get_version_detail, Library, OSName, get_version_assets};


#[cfg(target_os="windows")]
const ACTUAL_OS: OSName = OSName::Windows;
#[cfg(target_os="linux")]
const ACTUAL_OS: OSName = OSName::Linux;
#[cfg(target_os="macos")]
const ACTUAL_OS: OSName = OSName::MacOsX;

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
    pub version_number: String,
    pub version_type: VersionType,
    // version_custom: String, // for a next update
    pub memory_min: String,
    pub memory_max: String,
}

pub struct MinecraftClient<'a> {
    opts: &'a ClientOptions<'a>,
    details: VersionDetail,
    reqwest_client: Client,

}

impl<'a> MinecraftClient<'_> {
    pub async fn new(opts: &'a ClientOptions<'a>) -> Result<MinecraftClient<'a>> {
        let reqwest_client = Client::new();
        let details = Self::load_manifest(&reqwest_client, &opts).await?;
        Ok(MinecraftClient {
            opts,
            reqwest_client,
            details,
        })
    }

    async fn load_manifest(reqwest_client: &Client, opts: &ClientOptions<'a>) -> Result<VersionDetail> {
        let manifest = get_version_manifest(&reqwest_client).await?;
        let version = get_version_from_manifest(&manifest, opts.version_number.clone(), &opts.version_type).await?;
        let details = get_version_detail(&reqwest_client, version).await?;
        // let version_assets = get_version_assets(&details.asset_index).await?;
        Ok(details)
    }

    pub async fn download_requirements(&mut self) -> Result<()> {
        // create root folder if it doesn't exist
        if !self.opts.root_path.exists() {
            fs::create_dir_all(self.opts.root_path).await?;
        }
        let lib = &self.opts.root_path.join("libraries");
        if !lib.exists() {
            fs::create_dir(lib).await?;
        }
        self.download_libraries(lib).await?;
        self.opts.log_channel.closed().await;
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
            let mut file = if (&file_path).exists() {
                let f = fs::File::open(&file_path).await;
                match f {
                    Ok(f) => f,
                    Err(err) => bail!(err),
                }
            } else {
                fs::File::create(file_path).await?
            };
            
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

    /// Filter non necessary librairies for the current OS
    fn filter_non_necessary_librairies(&mut self) {
        self.details.libraries.retain(|e| { Self::should_use_library(e) });  
    }

    async fn download_assets(&mut self) -> Result<()> {
        bail!("Not yet implemented")
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
