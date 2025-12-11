use actix_web::{HttpResponse, web, Result as ActixResult};
use anyhow::{Context, Result};
use bytes::Bytes;
use image::{imageops::FilterType, GenericImageView, ImageFormat};
use log::{debug, error, info};
use serde::Deserialize;
use std::io::Cursor;

use crate::{ErrorResponse, routes::preview::HTTP_CLIENT};

pub async fn resize_from_url(
    url: &str,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Bytes> {
    let response = HTTP_CLIENT
        .get(url)
        .send()
        .await
        .context("Failed to fetch image")?;
    let image_bytes = response
        .bytes()
        .await
        .context("Failed to read image bytes")?;
    
    resize_bytes(image_bytes, width, height)
}

pub fn resize_bytes(
    image_bytes: Bytes,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Bytes> {
    let img = image::load_from_memory(&image_bytes)
        .context("Failed to decode image")?;
    let (original_width, original_height) = img.dimensions();
    debug!(
        "Original image dimensions: {}x{}",
        original_width, original_height
    );
    let (target_width, target_height) = calculate_dimensions(
        original_width,
        original_height,
        width,
        height,
    );
    debug!(
        "Resizing to: {}x{}",
        target_width, target_height
    );
    let resized = img.resize(target_width, target_height, FilterType::Lanczos3);
    let mut output = Cursor::new(Vec::new());
    resized
        .write_to(&mut output, ImageFormat::Png)
        .context("Failed to encode resized image")?;

    let output_bytes = Bytes::from(output.into_inner());
    debug!("Resized image size: {} bytes", output_bytes.len());
    Ok(output_bytes)
}

fn calculate_dimensions(
    original_width: u32,
    original_height: u32,
    target_width: Option<u32>,
    target_height: Option<u32>,
) -> (u32, u32) {
    match (target_width, target_height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => {
            let aspect_ratio = original_height as f32 / original_width as f32;
            let h = (w as f32 * aspect_ratio).round() as u32;
            (w, h.max(1))
        }
        (None, Some(h)) => {
            let aspect_ratio = original_width as f32 / original_height as f32;
            let w = (h as f32 * aspect_ratio).round() as u32;
            (w.max(1), h)
        }
        (None, None) => (original_width, original_height),
    }
}


#[derive(Deserialize)]
pub struct PreviewImageQuery {
    width: Option<u32>,
    height: Option<u32>,
    url: String,
}

pub async fn preview_image(query: web::Query<PreviewImageQuery>) -> ActixResult<HttpResponse> {
    info!(
        "Resizing image: {} ({}x{})",
        query.url,
        query.width.unwrap_or(0),
        query.height.unwrap_or(0)
    );
    if query.width.is_none() && query.height.is_none() {
        return Ok(HttpResponse::BadRequest().json(ErrorResponse {
            error: "At least one dimension (width or height) must be specified".to_string(),
        }));
    }
    match resize_from_url(&query.url, query.width, query.height).await {
        Ok(resized_bytes) => Ok(HttpResponse::Ok()
            .content_type("image/png")
            .body(resized_bytes)),
        Err(e) => {
            error!("Image resize error: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to resize image: {}", e),
            }))
        }
    }
}