use std::collections::HashMap;

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
    pub asset_index: AssetIndex,
    pub assets: String,
    #[serde(rename(serialize = "complianceLevel", deserialize = "complianceLevel"))]
    compliance_level: i32,
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
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: usize,
    #[serde(rename(serialize = "totalSize", deserialize = "totalSize"))]
    pub total_size: usize,
    pub url: String
}

#[derive(Serialize, Deserialize)]
pub struct Library {
    pub downloads: LibraryDownload,
    pub name: String,
    pub rules: Option<Vec<LibraryRule>>
}

#[derive(Serialize, Deserialize)]
pub struct LibraryDownload {
    pub artifact: LibraryArtifact
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
pub struct LibraryArtifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
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

#[derive(Serialize, Deserialize)]
pub struct AssetsManifest {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

pub async fn get_version_assets(reqwest: &Client , assets_index: &AssetIndex) -> Result<AssetsManifest> {
    let received: AssetsManifest = reqwest
        .get(assets_index.url.clone())
        .send()
        .await?
        .json()
        .await?;
    Ok(received)
}