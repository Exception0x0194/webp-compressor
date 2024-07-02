// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use image;
use rayon::prelude::*;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
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
fn get_folder_file_paths(dir_path: String) -> Result<Vec<String>, String> {
    // 定义支持的图片文件扩展名
    let supported_extensions = ["png", "jpg", "jpeg", "gif", "webp"];
    let mut file_paths = Vec::new();

    // 递归地搜索目录
    fn search_dir(
        path: PathBuf,
        file_paths: &mut Vec<String>,
        supported_extensions: &[&str],
    ) -> Result<(), String> {
        // 检查路径是否真的是一个目录
        if !path.is_dir() {
            return Err(format!("{} is not a directory", path.to_string_lossy()));
        }

        match fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries {
                    let entry = entry.map_err(|e| e.to_string())?;
                    let path = entry.path();
                    if path.is_dir() {
                        // 递归调用处理子目录
                        search_dir(path, file_paths, supported_extensions)?;
                    } else {
                        // 检查是否是文件并且扩展名是否在支持列表中
                        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            if supported_extensions.contains(&ext.to_lowercase().as_str()) {
                                // 将路径转换为字符串
                                match path.to_str() {
                                    Some(path_str) => file_paths.push(path_str.to_string()),
                                    None => {
                                        return Err("Failed to convert path to string".to_string())
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => return Err(format!("Failed to read directory: {}", e)),
        }

        Ok(())
    }

    // 开始递归搜索
    let start_path = PathBuf::from(dir_path);
    search_dir(start_path, &mut file_paths, &supported_extensions)?;

    Ok(file_paths)
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
            set_output_path,
            get_folder_file_paths
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
