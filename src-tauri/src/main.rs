// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use base64::{engine::general_purpose, Engine as _};
use image::DynamicImage;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use webp::Encoder;

#[derive(Serialize, Deserialize)]
struct ImageData {
    filename: String,
    content: String, // 这里将接收Base64编码的字符串
}

#[tauri::command]
fn compress_and_zip_images(images: Vec<ImageData>, quality: f32) -> Result<Vec<ImageData>, String> {
    let converted_images: Result<Vec<ImageData>, String> = images
        .par_iter() // 使用并行迭代器
        .map(|image| {
            let content = general_purpose::STANDARD
                .decode(&image.content)
                .map_err(|e| e.to_string())?;
            let img = image::load_from_memory(&content).map_err(|e| e.to_string())?;
            let webp_bytes = encode_image_to_webp(&img, quality)?;
            let encoded_webp = general_purpose::STANDARD.encode(webp_bytes);
            Ok(ImageData {
                filename: image.filename.clone(),
                content: encoded_webp,
            })
        })
        .collect();

    match converted_images {
        Ok(images) => Ok(images),
        Err(e) => Err(e),
    }
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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, compress_and_zip_images])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
