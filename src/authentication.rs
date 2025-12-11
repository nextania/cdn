use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use log::info;
use std::future::{ready, Ready};

use crate::database::get_session;

pub struct AuthenticationMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthenticationMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthenticationMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddlewareService { service }))
    }
}

pub struct AuthenticationMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let fut = self.service.call(req);

        Box::pin(async move {
            match auth_header {
                Some(token) => {
                    info!("Request with Authorization header");
                    
                    match validate_token(&token).await {
                        Ok(user_id) => {
                            // Store user_id in extensions for use in handlers
                            let req = fut.await?;
                            req.request().extensions_mut().insert(user_id);
                            Ok(req)
                        }
                        Err(e) => {
                            Err(e)
                        }
                    }
                }
                None => {
                    Err(actix_web::error::ErrorUnauthorized("Authorization header required"))
                }
            }
        })
    }
}

async fn validate_token(token: &str) -> Result<String, actix_web::Error> {
    let session = get_session(token).await.map_err(|e| {
        actix_web::error::ErrorUnauthorized(format!("Token validation error: {}", e))
    })?;
    let session = match session {
        None => {
            return Err(actix_web::error::ErrorUnauthorized("Invalid or expired token"));
        }
        Some(s) => s,
    };
    Ok(session.user_id)
}
