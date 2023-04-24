mod manifest;

use std::path::{Path, self};

use anyhow::{Result, bail};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use tokio::{fs, io::AsyncWriteExt};

use crate::authentification::GameProfile;

use self::manifest::{VersionDetail, get_version_manifest, get_version_from_manifest, get_version_detail, Library, OSName};


#[cfg(target_os="windows")]
const ACTUAL_OS: OSName = OSName::Windows;
#[cfg(target_os="linux")]
const ACTUAL_OS: OSName = OSName::Linux;
#[cfg(target_os="macos")]
const ACTUAL_OS: OSName = OSName::MacOsX;

pub struct ClientOptions<'a> {
    pub authorization: GameProfile,
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
        Ok(details)
    }

    pub async fn download_assets(&mut self) -> Result<()> {
        // create root folder if it doesn't exist
        if !self.opts.root_path.exists() {
            fs::create_dir_all(self.opts.root_path).await?;
        }
        let lib = &self.opts.root_path.join("libraries");
        if !lib.exists() {
            fs::create_dir(lib).await?;
        }
        self.filter_non_necessary_librairies();
        for (_, i) in self.details.libraries.iter().enumerate() {
            let p = i.downloads.artifact.path.clone();
            let mut splited = p.split("/").collect::<Vec<&str>>();
            let filename = splited.pop().ok_or(anyhow::anyhow!("Invalid filename"))?; // remove last element
            let p = splited.join(path::MAIN_SEPARATOR_STR);
            let p = &lib.join(p);
            fs::create_dir_all(p).await?;
            let url = i.downloads.artifact.url.clone();
            let content = self.reqwest_client
            .get(url)
            .send()
            .await?
            .bytes()
            .await?;
            /* let mut hasher = Sha1::new();
            hasher.update(&content);
            let sha1 = hasher.finalize().to_vec();
            if sha1 != i.downloads.artifact.sha1.as_bytes() {
                bail!("Sha1 {:?} of {} is invalid, a malicious file might be present on the remote server, should be {}", sha1, i.name, i.downloads.artifact.sha1)
            } */
            let file = p.join(filename);
            let mut file = fs::File::create(file).await?;
            file.write_all(&content).await?;
            println!("{} downloaded", i.name);
        }
        
        Ok(())
    }

    /// Filter non necessary librairies for the current OS
    fn filter_non_necessary_librairies(&mut self) {
        self.details.libraries.retain(|e| { Self::should_use_library(e) });  
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
