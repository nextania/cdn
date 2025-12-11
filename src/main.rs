use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpResponse, HttpServer};
use env_logger::Env;
use log::{error, info};
use serde::Serialize;

pub mod authentication;
pub mod clamav;
pub mod database;
pub mod signature;
pub mod routes;
pub mod environment;

use authentication::AuthenticationMiddleware;
use database::FileRepository;
use tokio::time::sleep;

use crate::environment::{BIND_ADDRESS, CLAMAV_HOST, CLAMAV_PORT, S3_BUCKET, S3_BUCKET_NAME};


#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

pub const SERVICE: &str = "cdn";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub fn get_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Unexpected error: time went backwards")
        .as_millis() as u64
}

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "service": SERVICE,
        "timestamp": get_time_millis(),
        "version": VERSION,
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));
    
    info!("Nextania CDN version {}", env!("CARGO_PKG_VERSION"));

    info!("Connecting to MongoDB...");
    database::connect().await;
    
    info!("S3 bucket: {}", &*S3_BUCKET_NAME);
    info!("ClamAV: {}:{}", &*CLAMAV_HOST, &*CLAMAV_PORT);
    
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(30 * 60)).await;
            info!("Running file cleanup task...");
            match FileRepository::find_expired_files().await {
                Ok(expired_files) => {
                    if expired_files.is_empty() {
                        info!("No expired files found");
                    } else {
                        info!("Found {} expired files to delete", expired_files.len());
                        for file in expired_files {
                            match S3_BUCKET.delete_object(&file.id).await {
                                Ok(_) => {
                                    info!("Deleted expired file {} from S3", file.id);
                                    if let Err(e) = FileRepository::delete_file(&file.id).await {
                                        error!("Failed to delete {} from MongoDB: {}", file.id, e);
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to delete expired file {} from S3: {}", file.id, e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to find expired files: {}", e);
                }
            }
        }
    });
    
    let server = HttpServer::new(move || {
        let mut app = App::new()
            .wrap(
                Cors::default()
                    .allowed_origin_fn(|_, _| true)
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials(),
            )
            .wrap(actix_web::middleware::Logger::default())
            .service(
                web::scope("/api")
                    .wrap(AuthenticationMiddleware) 
                    .route("/upload", web::post().to(routes::upload::upload_file))
                    .route("/preview", web::get().to(routes::preview::get_link_preview))
                    .route("/preview/image", web::get().to(routes::preview_image::preview_image))
            )
            .route("/files/{file_id}", web::get().to(routes::serve::serve_file))
            .route("/", web::get().to(health_check));
        let assets_path = Path::new("./assets");
        if assets_path.exists() && assets_path.is_dir() {
            info!("Serving static files from /assets");
            app = app.service(
                Files::new("/assets", "./assets")
            );
        }
        app
    });
    
    info!("Starting server on {}...", &*BIND_ADDRESS);
    server
        .bind(&*BIND_ADDRESS)?
        .run()
        .await
}
