use axum::{
    extract::Json,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use log::{error, info};

use crate::handle_sign_cert::{SignCertificateRequest, SignCertificateResponse};
use crate::errors::CertificateError;

pub async fn sign_certificate_route(
    Json(request): Json<SignCertificateRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Implementation goes here
    todo!()
}

#[derive(serde::Serialize)]
struct ErrorResponse {
    error: String,
}

pub fn get_routes() -> Router {
    Router::new()
        .route("/sign-certificate", post(sign_certificate_route))
}
