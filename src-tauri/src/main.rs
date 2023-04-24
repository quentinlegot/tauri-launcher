#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

pub mod authentification;
pub mod launcher;

use std::sync::Mutex;

use authentification::{Authentification, Prompt, GameProfile};
use anyhow::Result;
use directories::BaseDirs;
use launcher::{MinecraftClient, ClientOptions};

struct CustomState (Option<GameProfile>);

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn login(app: tauri::AppHandle, _window: tauri::Window, state: tauri::State<'_, Mutex<CustomState>>) -> Result<String, String> {
    let result = Authentification::login(Prompt::SelectAccount, app).await;
    match result {
        Ok(val) => {
            let name = val.name.clone();
            match state.lock() {
                Ok(mut game_profile) => {
                    game_profile.0 = Some(val);
                    Ok(format!("Hello {}", name))
                },
                Err(err) => {
                    Err(err.to_string())
                }
            }
        },
        Err(err) => Err(err.to_string())
    }
}

#[tauri::command]
async fn download(state: tauri::State<'_, Mutex<CustomState>>) -> Result<String, String> {
    if let Some(base_dir) = BaseDirs::new() {
        let data_folder = base_dir.data_dir().join(".altarik_test");
        let root_path = data_folder.as_path();
        let java_path = root_path.join("java");
        let game_profile = match state.lock() {
            Ok(res) => Ok(res.0.clone()),
            Err(err) => Err(err.to_string())
        }?;
        if game_profile.is_none() {
            return Err("You're not connected".to_string());
        }
        let opts = ClientOptions {
            authorization: game_profile.unwrap(),
            root_path,
            java_path: &java_path.as_path(),
            version_number: "1.19.4".to_string(),
            version_type: launcher::VersionType::Release,
            memory_min: "2G".to_string(),
            memory_max: "4G".to_string(),
        };
        let client = MinecraftClient::new(&opts).await;
        match client {
            Ok(mut client) => {
                match client.download_assets().await {
                    Ok(_) => {
                        Ok("Content downloaded".to_string())
                    },
                    Err(err) => {
                        Err(err.to_string())
                    }
                }
            },
            Err(err) => {
                Err(err.to_string())
            }
        }
        
    } else {
        Err("Cannot download files".to_string())
    }
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(CustomState(None)))
        .invoke_handler(tauri::generate_handler![greet, login, download])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
