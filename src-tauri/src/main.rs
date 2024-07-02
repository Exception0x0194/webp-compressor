// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use image;
use rayon::prelude::*;
use std::{fs, io::Write, path::Path};
use tauri::Window;
use webp::Encoder;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ImageResultData {
    file_name: String,
    original_size: usize,
    compressed_size: usize,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct AppConfig {
    output_path: Option<String>,
}

#[tauri::command]
async fn add_compress_path_list(
    path_list: Vec<String>,
    quality: f32,
    output_path: String,
    window: Window,
) -> Result<(), ()> {
    fs::create_dir_all(&output_path).map_err(|_| ())?;
    path_list.par_iter().for_each(|path| {
        match compress_and_encode_image(path, quality) {
            Ok((data, original_size, compressed_size)) => {
                let output_file_path = Path::new(&output_path)
                    .join(Path::new(path).file_stem().unwrap().to_str().unwrap())
                    .with_extension("webp");
                let mut output_file = fs::File::create(output_file_path.clone()).unwrap();
                output_file.write_all(&data).unwrap();

                // Prepare and send image result data
                let image_result = ImageResultData {
                    file_name: output_file_path.to_string_lossy().to_string(),
                    original_size,
                    compressed_size,
                };
                window.emit("singleTaskCompleted", image_result).unwrap();
            }
            Err(e) => eprintln!("Error processing image: {}", e),
        }
    });
    Ok(())
}

#[tauri::command]
fn set_output_path(app_handle: tauri::AppHandle, output_path: String) -> Result<(), String> {
    let config_dir = app_handle.path_resolver().app_config_dir().unwrap();
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?; // 确保配置目录存在
    let config_file_path = config_dir.join("config.json");

    let config = AppConfig {
        output_path: Some(output_path),
    };

    let config_data = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    std::fs::write(config_file_path, config_data).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn get_output_path(app_handle: tauri::AppHandle) -> Result<String, String> {
    let config_dir = app_handle.path_resolver().app_config_dir().unwrap();
    let config_file_path = config_dir.join("config.json");

    if config_file_path.exists() {
        let config_data = std::fs::read_to_string(config_file_path).map_err(|e| e.to_string())?;
        let config: AppConfig = serde_json::from_str(&config_data).map_err(|e| e.to_string())?;
        Ok(config.output_path.unwrap_or_default())
    } else {
        Ok(String::new()) // 如果配置文件不存在，返回空字符串
    }
}

fn compress_and_encode_image(path: &str, quality: f32) -> Result<(Vec<u8>, usize, usize), String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let rgba_image = img.to_rgba8();
    let width = img.width();
    let height = img.height();

    // Convert image to WebP and get the compressed data
    let encoder = Encoder::from_rgba(&rgba_image, width, height);
    let webp_data = encoder.encode(quality);

    // Get original and compressed sizes
    let original_size = fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;
    let compressed_size = webp_data.len();

    Ok((webp_data.to_vec(), original_size, compressed_size))
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            greet,
            add_compress_path_list,
            get_output_path,
            set_output_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
