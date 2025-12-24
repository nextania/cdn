use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn verify_signature(
    file_id: &str,
    secret_key: &str,
    signature: &str,
    timestamp: u64,
    expiry_seconds: u64,
) -> bool {
    // validate this signature against the current time
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if current_time > timestamp + expiry_seconds {
        return false;
    }
    // in case a future timestamp is signed
    if timestamp > current_time + 60 {
        return false;
    }
    let mut mac = match Hmac::<Sha256>::new_from_slice(secret_key.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(timestamp.to_be_bytes().as_ref());
    mac.update(file_id.as_bytes());
    let signature_bytes = match hex::decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    mac.verify_slice(&signature_bytes).is_ok()
}

pub fn generate_signature(file_id: &str, secret_key: &str) -> (String, u64) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let mut mac = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(timestamp.to_be_bytes().as_ref());
    mac.update(file_id.as_bytes());
    let result = mac.finalize();
    (hex::encode(result.into_bytes()), timestamp)
}
