use actix_web::{web, HttpResponse, Responder};

use crate::constants::CACHE_CONTROL;
use crate::database::File;
use crate::errors::Result;
use crate::stores::Store;
use crate::utilities::fetch_file;

pub async fn handle(path: web::Path<(String, String)>) -> Result<impl Responder> {
    let (store_id, id) = path.into_inner();
    Store::get(&store_id)?;
    let file = File::find(&id, &store_id).await?;
    let (contents, _) = fetch_file(&id, &store_id, file.metadata, None).await?;
    Ok(HttpResponse::Ok()
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", file.filename),
        ))
        .insert_header(("Cache-Control", CACHE_CONTROL))
        .content_type(file.content_type)
        .body(contents))
}