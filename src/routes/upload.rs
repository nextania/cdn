use actix_multipart::Multipart;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Result as ActixResult};
use bytes::Bytes;
use futures_util::StreamExt;
use log::{error, info, warn};
use serde::Serialize;
use ulid::Ulid;

use crate::{
    ErrorResponse, clamav,
    database::{FileDocument, FileRepository},
    environment::S3_BUCKET,
    signature,
};

pub const MAX_FILE_SIZE: u64 = 25 * 1024 * 1024; // 25MB in bytes

#[derive(Serialize)]
pub struct UploadResponse {
    id: String,
    size: u64,
    content_type: String,
    signature: String,
    serve_url: String,
}

pub async fn upload_file(req: HttpRequest, mut payload: Multipart) -> ActixResult<HttpResponse> {
    info!("Received file upload request");

    // Get user_id from request extensions (set by auth middleware)
    let user_id =
        req.extensions()
            .get::<String>()
            .cloned()
            .ok_or(actix_web::error::ErrorUnauthorized(
                "User ID not found in request",
            ))?;

    let mut file_data: Option<Bytes> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;

    while let Some(item) = payload.next().await {
        let mut field =
            item.map_err(|e| actix_web::error::ErrorBadRequest(format!("Multipart error: {}", e)))?;

        let content_disposition = field.content_disposition();
        let field_name = content_disposition.as_ref().and_then(|cd| cd.get_name());

        if field_name == Some("file") {
            file_name = content_disposition
                .as_ref()
                .and_then(|cd| cd.get_filename())
                .map(|s| s.to_string());
            content_type = field.content_type().map(|ct| ct.to_string());

            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                let data = chunk
                    .map_err(|e| actix_web::error::ErrorBadRequest(format!("Read error: {}", e)))?;
                bytes.extend_from_slice(&data);
            }
            file_data = Some(Bytes::from(bytes));
            break;
        }
    }
    let file_data =
        file_data.ok_or_else(|| actix_web::error::ErrorBadRequest("No file provided"))?;
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());
    let file_size = file_data.len() as u64;
    if file_size > MAX_FILE_SIZE {
        warn!("File size {} exceeds limit of {}", file_size, MAX_FILE_SIZE);
        return Ok(HttpResponse::PayloadTooLarge().json(ErrorResponse {
            error: format!(
                "File size exceeds maximum allowed size of 25MB (received {} bytes)",
                file_size
            ),
        }));
    }

    info!("Scanning file with ClamAV");
    match clamav::scan_bytes(&file_data).await {
        Ok(true) => info!("File is clean"),
        Ok(false) => {
            error!("File is infected");
            return Ok(HttpResponse::BadRequest().json(ErrorResponse {
                error: "File is infected with malware".to_string(),
            }));
        }
        Err(e) => {
            error!("ClamAV scan error: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Virus scan failed: {}", e),
            }));
        }
    }

    let file_id = Ulid::new().to_string();
    info!("Uploading file to S3: {}", file_id);
    match S3_BUCKET
        .put_object_with_content_type(&file_id, &file_data, &content_type)
        .await
    {
        Ok(_) => {
            info!("File uploaded successfully: {}", file_id);
            let file_doc = FileDocument::new(
                file_id.clone(),
                file_name,
                content_type.clone(),
                file_size,
                user_id.clone(),
            );
            let (signature, timestamp) =
                signature::generate_signature(&file_id, &file_doc.signing_key);
            let serve_url = format!(
                "/files/{}?signature={}&timestamp={}",
                file_id, signature, timestamp
            );
            if let Err(e) = FileRepository::insert_file(file_doc).await {
                error!("Failed to save file metadata to MongoDB: {}", e);
                // TODO: delete the file from S3 here to avoid orphaned files?
                warn!("File {} uploaded to S3 but not tracked in MongoDB", file_id);
            }
            Ok(HttpResponse::Ok().json(UploadResponse {
                id: file_id,
                size: file_size,
                content_type,
                signature,
                serve_url,
            }))
        }
        Err(e) => {
            error!("S3 upload error: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Upload failed: {}", e),
            }))
        }
    }
}
