use lazy_static::lazy_static;
use s3::{Bucket, Region, creds::Credentials};

lazy_static! {
    pub static ref S3_REGION: String = std::env::var("S3_REGION").unwrap_or_default();
    pub static ref S3_ENDPOINT: String =
        std::env::var("S3_ENDPOINT").expect("S3_ENDPOINT must be set");
    pub static ref S3_BUCKET_NAME: String =
        std::env::var("S3_BUCKET_NAME").expect("S3_BUCKET_NAME must be set");
    pub static ref S3_ACCESS_KEY: String =
        std::env::var("S3_ACCESS_KEY").expect("S3_ACCESS_KEY must be set");
    pub static ref S3_SECRET_KEY: String =
        std::env::var("S3_SECRET_KEY").expect("S3_SECRET_KEY must be set");
    pub static ref S3_BUCKET: Bucket = {
        let credentials =
            Credentials::new(Some(&S3_ACCESS_KEY), Some(&S3_SECRET_KEY), None, None, None)
                .expect("Failed to create S3 credentials");

        let region = Region::Custom {
            region: S3_REGION.clone(),
            endpoint: S3_ENDPOINT.clone(),
        };

        *Bucket::new(&S3_BUCKET_NAME, region, credentials).expect("Failed to create S3 bucket")
    };
    pub static ref CLAMAV_HOST: String =
        std::env::var("CLAMAV_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    pub static ref CLAMAV_PORT: u16 = std::env::var("CLAMAV_PORT")
        .unwrap_or_else(|_| "3310".to_string())
        .parse::<u16>()
        .expect("CLAMAV_PORT must be a valid port number");
    pub static ref BIND_ADDRESS: String =
        std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    pub static ref MONGODB_URI: String =
        std::env::var("MONGODB_URI").expect("MONGODB_URI must be set");
    pub static ref MONGODB_DATABASE: String =
        std::env::var("MONGODB_DATABASE").expect("MONGODB_DATABASE must be set");
    pub static ref AS_MONGODB_DATABASE: String =
        std::env::var("AS_MONGODB_DATABASE").expect("AS_MONGODB_DATABASE must be set");
    pub static ref FILE_TIMEOUT_HOURS: i64 = std::env::var("FILE_TIMEOUT_HOURS")
        .unwrap_or_else(|_| "3".to_string())
        .parse::<i64>()
        .expect("FILE_TIMEOUT_HOURS must be a valid number");
    pub static ref SIGNATURE_EXPIRY_SECONDS: u64 = std::env::var("SIGNATURE_EXPIRY_SECONDS")
        .unwrap_or_else(|_| "3600".to_string())
        .parse::<u64>()
        .expect("SIGNATURE_EXPIRY_SECONDS must be a valid number");
}
