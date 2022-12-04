#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

pub mod authentification;

use authentification::{Authentification, Prompt};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn second_window(app: tauri::AppHandle, window: tauri::Window) -> Result<(), String> {
    Authentification::launch(Prompt::SelectAccount, app);
  Ok(())
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, second_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
