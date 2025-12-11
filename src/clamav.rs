use anyhow::{Context, Result};
use bytes::Bytes;
use clamav_client;
use log::debug;

use crate::environment::{CLAMAV_HOST, CLAMAV_PORT};

pub async fn scan_bytes(data: &Bytes) -> Result<bool> {
    let address = format!("{}:{}", &*CLAMAV_HOST, &*CLAMAV_PORT);
    let data_owned = data.clone();

    debug!("Sending {} bytes to ClamAV for scanning", data_owned.len());
    let clamd = clamav_client::tokio::Tcp {
        host_address: address,
    };
    let response = clamav_client::tokio::scan_buffer(&data_owned, clamd, None)
        .await
        .context("Failed to scan file with ClamAV")?;
    let response_str = String::from_utf8_lossy(&response);
    debug!("ClamAV response: {}", response_str);

    let is_clean = clamav_client::clean(&response).context("Failed to parse ClamAV response")?;
    Ok(is_clean)
}
