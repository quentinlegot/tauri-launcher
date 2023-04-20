use std::{fmt::Display, path::{self, Path}};

use anyhow::{Result, bail};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use serde_json::{Value, Map};
use tokio::fs;

use crate::authentification::GameProfile;

#[derive(Serialize, Deserialize, Debug)]
pub struct VersionManifestV2 {
    latest: Value,
    versions: Vec<Version>
}

#[derive(Serialize, Deserialize, Debug)]
struct Version {
    id: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    v_type: VersionType,
    url: String,
    sha1: String
}


async fn get_version_manifest(reqwest: &Client) -> Result<VersionManifestV2> {
    let received: VersionManifestV2 = reqwest
    .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
    .send()
    .await?
    .json()
    .await?;
    Ok(received)
}

fn get_version_from_manifest<'a>(manifest: &'a VersionManifestV2, game_version: String, version_type: &VersionType) -> Result<&'a Version> {
    for i in manifest.versions.iter().enumerate() {
        let id = i.1.id.clone();
        let v_type = i.1.v_type;
        if id == game_version && &v_type == version_type {
            return Ok(i.1);
        }
    }
    bail!("Version not Found")
}

#[derive(Serialize, Deserialize)]
struct VersionDetail {
    arguments: Map<String, Value>,
    #[serde(rename(serialize = "assetIndex", deserialize = "assetIndex"))]
    asset_index: Map<String, Value>,
    assets: String,
    downloads: Map<String, Value>,
    id: String,
    #[serde(rename(serialize = "javaVersion", deserialize = "javaVersion"))]
    java_version: Map<String, Value>,
    libraries: Vec<Library>,
    logging: Map<String, Value>,
    #[serde(rename(serialize = "mainClass", deserialize = "mainClass"))]
    main_class: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    v_type: VersionType
}

#[derive(Serialize, Deserialize)]
struct Library {
    downloads: LibraryDownload,
    name: String,
    rules: Vec<LibraryRule>
}

#[derive(Serialize, Deserialize)]
struct LibraryRule {
    action: String,
    os: LibraryOSRule
}
#[derive(Serialize, Deserialize)]
struct LibraryOSRule {
    name: OSName,
}

#[derive(Serialize, Deserialize)]
enum OSName {
    #[serde(rename(serialize = "osx", deserialize = "osx"))]
    MacOsX,
    #[serde(rename(serialize = "linux", deserialize = "linux"))]
    Linux,
    #[serde(rename(serialize = "windows", deserialize = "windows"))]
    Windows
}

#[derive(Serialize, Deserialize)]
struct LibraryDownload {
    artifact: LibraryArtifact
}

#[derive(Serialize, Deserialize)]
struct LibraryArtifact {
    path: String,
    sha1: String,
    size: i64,
    url: String,
}

async fn get_version_detail(reqwest: &Client, version : &Version) -> Result<VersionDetail> {
    let received: VersionDetail = reqwest
    .get(version.url.clone())
    .send()
    .await?
    .json()
    .await?;
    Ok(received)
}
pub struct ClientOptions<'a> {
    authorization: GameProfile,
    root_path: &'a Path,
    javaPath: String,
    version_number: String,
    version_type: VersionType,
    // version_custom: String, // for a next update
    memory_min: String,
    memory_max: String,
}

pub struct MinecraftClient<'a> {
    opts: ClientOptions<'a>,
    details: VersionDetail,
    reqwest_client: Client,

}

impl<'a> MinecraftClient<'_> {
    pub async fn new(opts: ClientOptions<'a>) -> Result<MinecraftClient<'a>> {
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
        let version = get_version_from_manifest(&manifest, opts.version_number.clone(), &opts.version_type)?;
        let details = get_version_detail(&reqwest_client, version).await?;
        Ok(details)
    }

    pub async fn download_assets(&self) -> Result<()> {
        // create root folder if it doesn't exist
        fs::create_dir_all(self.opts.root_path).await?;
        fs::create_dir(self.opts.root_path.join("librairies")).await?;
        
        Ok(())
    }
    /// Filter non necessary librairies for the current OS
    fn filter_non_necessary_librairies(&self) -> Result<()> {
        bail!("Not implemented yet")
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

impl<'a> TryInto<&'a VersionType> for &str {
    type Error = ();

    fn try_into(self) -> std::result::Result<&'a VersionType, Self::Error> {
        match self {
            "release" => Ok(&VersionType::Release),
            "snapshot" => Ok(&VersionType::Snapshot),
            "old_alpha" => Ok(&VersionType::OldAlpha),
            "old_beta" => Ok(&VersionType::OldBeta),
            _ => Err(()),
        }
    }

    
}

impl Display for VersionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::Release => "release",
            Self::Snapshot => "snapshot",
            Self::OldAlpha => "old_alpha",
            Self::OldBeta => "old_beta",
        };
        write!(f, "{}", str)
    }
}
