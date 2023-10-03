use anyhow::Result;
use reqwest::Client;
use serde::{Serialize, Deserialize};


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