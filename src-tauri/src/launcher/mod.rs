use std::fmt::Display;

use anyhow::{Result, bail, anyhow};
use reqwest::Client;
use serde_json::{Value, Map};



async fn get_version_manifest(reqwest: &Client) -> Result<Value> {
    let received: Value = reqwest
    .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
    .send()
    .await?
    .json()
    .await?;
    Ok(received)
}

fn get_version_from_manifest<'a>(manifest: &'a Value, game_version: String, version_type: &VersionType) -> Result<&'a Map<String, Value>> {
    let versions = manifest.get("versions").ok_or(anyhow!("Manifest format is invalid"))?;
    let arr = versions.as_array().ok_or(anyhow!("Manifest format is invalid"))?;
    for i in arr.iter().enumerate() {
        let map = i.1.as_object().ok_or(anyhow!("Manifest format is invalid"))?;
        let id = map.get("id").ok_or(anyhow!("Manifest format is invalid, cannot find version id"))?;
        let id = id.as_str().ok_or(anyhow!("Manifest format is invalid, id is not a str"))?;
        let v_type = map.get("type").ok_or(anyhow!("Manifest format is invalid, cannot find version type"))?;
        let v_type = v_type.as_str().ok_or(anyhow!("Manifest format is invalid, type is not a str"))?;
        if id == game_version && v_type.try_into() == Ok(version_type) {
            return Ok(map);
        }
    }
    bail!("Version not Found")
}

fn get_version_detail(reqwest: &Client, version : &Map<String, Value>) -> Result<()> {
    bail!("Not implemented yet")
}

pub async fn download_assets(game_version: String, version_type: &VersionType) -> Result<()> {
    let reqwest_client = Client::new();
    let manifest = get_version_manifest(&reqwest_client).await?;
    let version = get_version_from_manifest(&manifest, game_version, version_type)?;
    Ok(())
    
    
}

#[derive(PartialEq)]
pub enum VersionType {
    Release,
    Snapshot,
    OldAlpha,
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