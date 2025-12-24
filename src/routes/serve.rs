use actix_web::{HttpResponse, Result as ActixResult, web};
use log::{error, info, warn};
use serde::Deserialize;

use crate::{ErrorResponse, database::FileRepository, environment::{S3_BUCKET, SIGNATURE_EXPIRY_SECONDS}, signature};

#[derive(Deserialize)]
pub struct FileServeQuery {
    signature: String,
    timestamp: u64,
}

pub async fn serve_file(
    path: web::Path<String>,
    query: web::Query<FileServeQuery>,
) -> ActixResult<HttpResponse> {
    let file_id = path.into_inner();
    info!("Serving file request for: {}", file_id);
    let file_doc = match FileRepository::get_file(&file_id).await {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            error!("File not found: {}", file_id);
            return Ok(HttpResponse::NotFound().json(ErrorResponse {
                error: "File not found".to_string(),
            }));
        }
        Err(e) => {
            error!("MongoDB error: {}", e);
            return Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Database error".to_string(),
            }));
        }
    };
    if !signature::verify_signature(&file_id, &file_doc.signing_key, &query.signature, query.timestamp, *SIGNATURE_EXPIRY_SECONDS) {
        warn!("Invalid or expired signature for file: {}", file_id);
        return Ok(HttpResponse::Forbidden().json(ErrorResponse {
            error: "Invalid or expired signature".to_string(),
        }));
    }
    match S3_BUCKET.get_object(&file_doc.id).await {
        Ok(response) => {
            let bytes = response.bytes();
            info!(
                "File fetched successfully: {} ({} bytes)",
                file_id,
                bytes.len()
            );
            Ok(HttpResponse::Ok()
                .content_type(file_doc.content_type.as_str())
                .insert_header((
                    "Content-Disposition",
                    format!(
                        "inline; filename=\"{}\"",
                        file_doc.name.unwrap_or_else(|| file_doc.id.clone())
                    ),
                ))
                .body(bytes.to_vec()))
        }
        Err(e) => {
            error!("S3 fetch error for {}: {}", file_doc.id, e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to fetch file".to_string(),
            }))
        }
    }
}
