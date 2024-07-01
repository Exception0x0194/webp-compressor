// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose, Engine as _};
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use tokio;
use webp::Encoder;

#[derive(Serialize, Deserialize)]
struct ImageTaskData {
    file_name: String,
    content: String,
    quality: f32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ImageResultData {
    file_name: String,
    content: String,
}

#[tauri::command]
async fn add_compress_image_task(task: ImageTaskData, window: tauri::Window) -> Result<(), String> {
    tokio::spawn(async move {
        let result = get_image_compression_result(&task);
        match result {
            Ok(compressed_image) => {
                // 使用 Tauri 的 emit 将压缩后的图片发送到前端
                if let Err(e) = window.emit("imageProcessed", &compressed_image) {
                    eprintln!("Failed to emit imageProcessed event: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error processing image: {}", e);
                // 可以考虑向前端发送错误信息
            }
        }
    });

    Ok(())
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn encode_image_to_webp(img: &DynamicImage, quality: f32) -> Result<Vec<u8>, String> {
    let rgba_image = img.to_rgba8();
    let encoder = Encoder::from_rgba(&rgba_image, img.width(), img.height());
    Ok(encoder.encode(quality).to_vec())
}

fn get_image_compression_result(input: &ImageTaskData) -> Result<ImageResultData, String> {
    // 解码 Base64 编码的图片内容
    let image_data = general_purpose::STANDARD
        .decode(&input.content)
        .map_err(|e| format!("Failed to decode Base64 content: {}", e))?;

    // 加载图片
    let img = image::load_from_memory(&image_data)
        .map_err(|e| format!("Failed to load image from bytes: {}", e))?;

    // 压缩并编码图片为 WebP
    let compressed_image = encode_image_to_webp(&img, input.quality)?;

    // 将压缩后的图片数据编码为 Base64
    let base64_encoded_image = general_purpose::STANDARD.encode(&compressed_image);

    // 创建返回结果
    Ok(ImageResultData {
        file_name: input.file_name.clone(),
        content: base64_encoded_image,
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, add_compress_image_task])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
