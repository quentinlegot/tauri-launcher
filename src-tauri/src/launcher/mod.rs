mod manifest;

use std::path::Path;

use anyhow::Result;
use reqwest::Client;
use serde::{Serialize, Deserialize};
use tokio::fs;

use self::manifest::{VersionDetail, get_version_manifest, get_version_from_manifest, get_version_detail, Library, OSName};


#[cfg(target_os="windows")]
const ACTUAL_OS: OSName = OSName::Windows;
#[cfg(target_os="linux")]
const ACTUAL_OS: OSName = OSName::Linux;
#[cfg(target_os="macos")]
const ACTUAL_OS: OSName = OSName::MacOsX;

pub struct ClientOptions<'a> {
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
        let lib = self.opts.root_path.join("librairies");
        if !lib.exists() {
            fs::create_dir(lib).await?;
        }
        self.filter_non_necessary_librairies();
        
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
