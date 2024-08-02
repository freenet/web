use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
    Router,
};
use log::{error, info};

use crate::handle_sign_cert::{SignCertificateRequest, SignCertificateResponse};
use crate::errors::CertificateError;

pub async fn sign_certificate_route(
    Json(request): Json<SignCertificateRequest>,
) -> Result<Json<SignCertificateResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Implementation goes here
    todo!()
}

#[derive(serde::Serialize)]
struct ErrorResponse {
    error: String,
}

pub fn get_routes() -> Router {
    Router::new()
        .route("/sign-certificate", axum::routing::post(sign_certificate_route))
}
