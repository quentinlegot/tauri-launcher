use anyhow::{Result, bail};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use serde_json::{Value, Map};

use super::VersionType;

#[derive(Serialize, Deserialize, Debug)]
pub struct VersionManifestV2 {
    latest: Value,
    versions: Vec<Version>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Version {
    id: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    v_type: VersionType,
    url: String,
    sha1: String
}


pub async fn get_version_manifest(reqwest: &Client) -> Result<VersionManifestV2> {
    let received: VersionManifestV2 = reqwest
    .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
    .send()
    .await?
    .json()
    .await?;
    Ok(received)
}

pub async fn get_version_from_manifest<'a>(manifest: &'a VersionManifestV2, game_version: String, version_type: &VersionType) -> Result<&'a Version> {
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
pub struct VersionDetail {
    arguments: Map<String, Value>,
    #[serde(rename(serialize = "assetIndex", deserialize = "assetIndex"))]
    asset_index: Map<String, Value>,
    assets: String,
    downloads: Map<String, Value>,
    id: String,
    #[serde(rename(serialize = "javaVersion", deserialize = "javaVersion"))]
    java_version: Map<String, Value>,
    pub libraries: Vec<Library>,
    logging: Map<String, Value>,
    #[serde(rename(serialize = "mainClass", deserialize = "mainClass"))]
    main_class: String,
    #[serde(rename(serialize = "type", deserialize = "type"))]
    v_type: VersionType
}

#[derive(Serialize, Deserialize)]
pub struct Library {
    pub downloads: LibraryDownload,
    pub name: String,
    pub rules: Option<Vec<LibraryRule>>
}

#[derive(Serialize, Deserialize)]
pub struct LibraryDownload {
    artifact: LibraryArtifact
}

#[derive(Serialize, Deserialize)]
pub struct LibraryRule {
    pub action: String,
    pub os: LibraryOSRule
}
#[derive(Serialize, Deserialize)]
pub struct LibraryOSRule {
    pub name: OSName,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub enum OSName {
    #[serde(rename(serialize = "osx", deserialize = "osx"))]
    MacOsX,
    #[serde(rename(serialize = "linux", deserialize = "linux"))]
    Linux,
    #[serde(rename(serialize = "windows", deserialize = "windows"))]
    Windows
}

#[derive(Serialize, Deserialize)]
struct LibraryArtifact {
    path: String,
    sha1: String,
    size: i64,
    url: String,
}

pub async fn get_version_detail(reqwest: &Client, version : &Version) -> Result<VersionDetail> {
    let received: VersionDetail = reqwest
    .get(version.url.clone())
    .send()
    .await?
    .json()
    .await?;
    Ok(received)
}