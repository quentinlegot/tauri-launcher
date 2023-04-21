mod manifest;

use std::{path::Path, fs};

use anyhow::Result;
use reqwest::blocking::Client;
use serde::{Serialize, Deserialize};

use crate::authentification::GameProfile;

use self::manifest::{VersionDetail, get_version_manifest, get_version_from_manifest, get_version_detail, Library, OSName};


#[cfg(target_os="windows")]
const ACTUAL_OS: OSName = OSName::Windows;
#[cfg(target_os="linux")]
const ACTUAL_OS: OSName = OSName::Linux;
#[cfg(target_os="macos")]
const ACTUAL_OS: OSName = OSName::MacOsX;

pub struct ClientOptions<'a> {
    pub authorization: &'a GameProfile,
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
    pub fn new(opts: &'a ClientOptions<'a>) -> Result<MinecraftClient<'a>> {
        let reqwest_client = Client::new();
        let details = Self::load_manifest(&reqwest_client, &opts)?;
        Ok(MinecraftClient {
            opts,
            reqwest_client,
            details,
        })
    }

    fn load_manifest(reqwest_client: &Client, opts: &ClientOptions<'a>) -> Result<VersionDetail> {
        let manifest = get_version_manifest(&reqwest_client)?;
        let version = get_version_from_manifest(&manifest, opts.version_number.clone(), &opts.version_type)?;
        let details = get_version_detail(&reqwest_client, version)?;
        Ok(details)
    }

    pub fn download_assets(&mut self) -> Result<()> {
        // create root folder if it doesn't exist
        fs::create_dir_all(self.opts.root_path)?;
        fs::create_dir(self.opts.root_path.join("librairies"))?;
        self.filter_non_necessary_librairies();
        Ok(())
    }
    /// Filter non necessary librairies for the current OS
    fn filter_non_necessary_librairies(&mut self) {
        self.details.libraries.retain(|e| { Self::should_use_library(e) });  
    }

    fn should_use_library(library: &Library) -> bool {
        if library.rules.is_empty() {
            true
        } else {
            for i in library.rules.iter().enumerate() {
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
