use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn verify_signature(file_id: &str, secret_key: &str, signature: &str) -> bool {
    let mut mac = match Hmac::<Sha256>::new_from_slice(secret_key.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(file_id.as_bytes());
    let signature_bytes = match hex::decode(signature) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    mac.verify_slice(&signature_bytes).is_ok()
}

pub fn generate_signature(file_id: &str, secret_key: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(file_id.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}
