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
use launcher::{MinecraftClient, ClientOptions, ProgressMessage, altarik::{AltarikManifest, Chapter}};
use reqwest::Client;
use tauri::Manager;
use tokio::sync::mpsc;

struct CustomState (Option<GameProfile>, Option<AltarikManifest>);

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
async fn load_altarik_manifest(state: tauri::State<'_, Mutex<CustomState>>) -> Result<AltarikManifest, String> {
    let reqwest_client = Client::new();
    let altarik_manifest = AltarikManifest::get_altarik_manifest(&reqwest_client).await;
    match altarik_manifest {
        Ok(val) => {
            match state.lock() {
                Ok(mut global_manifest) => {
                    let _ = global_manifest.1.insert(val.clone());
                    Ok(val)
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
    
}

#[tauri::command]
async fn download(selected_chapter: usize, app: tauri::AppHandle, state: tauri::State<'_, Mutex<CustomState>>) -> Result<String, String> {
    let chapter = match state.lock() {
        Ok(lock) => {
            match &lock.1 {
                Some(manifest) => {
                    match manifest.chapters.get(selected_chapter) {
                        Some(val) => {
                            val.clone()
                        },
                        None => return Err("Selected chapter doesn't exist".to_string())
                    }
                },
                None => return Err("Cannot load altarik manifest".to_string())
            }
        },
        Err(err) => return Err(err.to_string())
    };
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
        let (sender,  receiver) = mpsc::channel(60);
        let opts = ClientOptions {
            authorization: game_profile.unwrap(),
            log_channel: sender.clone(),
            root_path,
            java_path: &java_path.as_path(),
            version_number: chapter.minecraft_version.clone(),
            version_type: launcher::VersionType::Release,
            memory_min: "2G".to_string(),
            memory_max: "4G".to_string(),
        };
        drop(sender);
        let res = tokio::join!(
            download_libraries(opts, chapter),
            read_channel(receiver, app),
        );
        res.0
    } else {
        Err("Cannot download files".to_string())
    }
}


async fn download_libraries(opts: ClientOptions<'_>, chapter: Chapter) -> Result<String, String> {
    let client = MinecraftClient::new(&opts).await;
    let res = match client {
        Ok(mut client) => {
            match client.download_requirements(chapter).await {
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
    };
    res
}

async fn read_channel(mut receiver: mpsc::Receiver<ProgressMessage>, app: tauri::AppHandle) -> Result<()> {
    loop {
        match receiver.recv().await {
            Some(msg) => { app.emit_all("progress", msg)? },
            None => break Ok(())
        }
    }
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(CustomState(None, None)))
        .invoke_handler(tauri::generate_handler![login, download, load_altarik_manifest])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
