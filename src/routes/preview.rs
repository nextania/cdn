use actix_web::{HttpResponse, Result as ActixResult, web};
use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::{error, info};
use reqwest;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use crate::ErrorResponse;

lazy_static! {
    pub static ref HTTP_CLIENT: reqwest::Client = {
        reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client")
    };
}

#[derive(Debug, Serialize)]
pub struct LinkPreview {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub site_name: Option<String>,
}

pub async fn fetch_preview(url: &str) -> Result<LinkPreview> {
    let response = HTTP_CLIENT
        .get(url)
        .send()
        .await
        .context("Failed to fetch URL")?;
    let html = response
        .text()
        .await
        .context("Failed to read response body")?;

    let document = Html::parse_document(&html);
    let title = extract_meta_content(&document, &["og:title", "twitter:title"])
        .or_else(|| extract_title(&document));
    let description = extract_meta_content(
        &document,
        &["og:description", "twitter:description", "description"],
    );
    let image = extract_meta_content(&document, &["og:image", "twitter:image"])
        .map(|img| resolve_url(url, &img));
    let site_name = extract_meta_content(&document, &["og:site_name", "twitter:site"]);

    Ok(LinkPreview {
        url: url.to_string(),
        title,
        description,
        image,
        site_name,
    })
}

fn extract_meta_content(document: &Html, properties: &[&str]) -> Option<String> {
    for property in properties {
        let selector_str = format!(r#"meta[property="{}"]"#, property);
        if let Ok(selector) = Selector::parse(&selector_str)
            && let Some(element) = document.select(&selector).next()
            && let Some(content) = element.value().attr("content")
            && !content.trim().is_empty()
        {
            return Some(content.to_string());
        }
        let selector_str = format!(r#"meta[name="{}"]"#, property);
        if let Ok(selector) = Selector::parse(&selector_str)
            && let Some(element) = document.select(&selector).next()
            && let Some(content) = element.value().attr("content")
            && !content.trim().is_empty()
        {
            return Some(content.to_string());
        }
    }
    None
}

fn extract_title(document: &Html) -> Option<String> {
    let selector = Selector::parse("title").ok()?;
    document
        .select(&selector)
        .next()
        .map(|element| element.inner_html().trim().to_string())
}

fn resolve_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }
    if let Ok(base_url) = url::Url::parse(base)
        && let Ok(resolved) = base_url.join(relative)
    {
        return resolved.to_string();
    }
    relative.to_string()
}

#[derive(Deserialize)]
pub struct LinkPreviewQuery {
    url: String,
}

pub async fn get_link_preview(query: web::Query<LinkPreviewQuery>) -> ActixResult<HttpResponse> {
    info!("Fetching link preview for: {}", query.url);
    match fetch_preview(&query.url).await {
        Ok(preview) => Ok(HttpResponse::Ok().json(preview)),
        Err(e) => {
            error!("Link preview error: {}", e);
            Ok(HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to fetch preview: {}", e),
            }))
        }
    }
}
