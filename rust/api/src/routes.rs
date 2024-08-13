use std::collections::HashMap;
use std::str::FromStr;

use axum::{
    routing::{get, post},
    Router,
    http::StatusCode,
    response::{IntoResponse, Json},
    extract::Path,
};
use log::{error, info};
use serde::{Deserialize, Serialize};
use stripe::{Client, Currency, PaymentIntent, PaymentIntentId};
use ghostkey_lib::armorable::Armorable;

use crate::delegates::get_delegate;
use crate::handle_sign_cert::{CertificateError, sign_certificate, SignCertificateRequest, SignCertificateResponse};

#[derive(Serialize)]
pub struct ErrorResponse {
    error: String,
    status: u16,
}

#[derive(Serialize, Deserialize)]
struct Message {
    content: String,
}

#[derive(Deserialize, Debug)]
pub struct DonationRequest {
    pub currency: String,
    pub amount: i64,
}

#[derive(Serialize)]
pub struct DonationResponse {
    pub client_secret: String,
    pub payment_intent_id: String,
    pub delegate_certificate_base64: String,
}

async fn index() -> impl IntoResponse {
    Json(serde_json::json!({
        "message": "Hello, world!"
    }))
}

async fn get_message() -> impl IntoResponse {
    Json(Message {
        content: String::from("Welcome to the Freenet API! This message confirms that the API is functioning correctly."),
    })
}

async fn sign_certificate_route(
    Json(request): Json<SignCertificateRequest>,
) -> Result<Json<SignCertificateResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Received sign-certificate request: {:?}", request);
    match sign_certificate(request).await {
        Ok(response) => {
            info!("Certificate signed successfully");
            Ok(Json(response))
        },
        Err(e) => {
            error!("Error signing certificate: {:?}", e);
            match e {
                CertificateError::PaymentNotSuccessful => {
                    Err((StatusCode::BAD_REQUEST, Json(ErrorResponse {
                        error: "Payment not successful. Please check your payment details and try again.".to_string(),
                        status: StatusCode::BAD_REQUEST.as_u16(),
                    })))
                },
                CertificateError::CertificateAlreadySigned => {
                    Err((StatusCode::CONFLICT, Json(ErrorResponse {
                        error: "Certificate has already been signed for this payment.".to_string(),
                        status: StatusCode::CONFLICT.as_u16(),
                    })))
                },
                CertificateError::KeyError(msg) => {
                    Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                        error: format!("Key error: {}", msg),
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    })))
                },
                _ => {
                    Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse {
                        error: "An unexpected error occurred. Please try again later.".to_string(),
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    })))
                }
            }
        },
    }
}

#[derive(Debug)]
pub enum DonationError {
    InvalidCurrency,
    StripeError(stripe::StripeError),
    EnvError(std::env::VarError),
    OtherError(String),
}

impl IntoResponse for DonationError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            DonationError::InvalidCurrency => (StatusCode::BAD_REQUEST, "Invalid currency"),
            DonationError::StripeError(e) => {
                error!("Stripe error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Stripe error occurred")
            },
            DonationError::EnvError(e) => {
                error!("Environment variable error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Environment variable error")
            },
            DonationError::OtherError(e) => {
                error!("Other error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "An unexpected error occurred")
            },
        };

        let body = Json(ErrorResponse {
            error: error_message.to_string(),
            status: status.as_u16(),
        });

        (status, body).into_response()
    }
}

async fn create_donation(
    Json(request): Json<DonationRequest>,
) -> Result<Json<DonationResponse>, DonationError> {
    info!("Received create-donation request: {:?}", request);
    
    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(DonationError::EnvError)?;
    let client = Client::new(&secret_key);

    let currency = Currency::from_str(&request.currency).map_err(|_| {
        error!("Invalid currency: {}", request.currency);
        DonationError::InvalidCurrency
    })?;

    let mut metadata = HashMap::new();
    metadata.insert("donation_type".to_string(), "freenet".to_string());

    let params = stripe::CreatePaymentIntent {
        amount: request.amount,
        currency,
        automatic_payment_methods: None,
        metadata: Some(metadata),
        description: Some("Freenet Donation"),
        statement_descriptor: Some("Freenet Donation"),
        statement_descriptor_suffix: Some("Thank You"),
        payment_method_types: Some(vec!["card".to_string()]),
        capture_method: None,
        confirm: None,
        setup_future_usage: None,
        confirmation_method: None,
        customer: None,
        error_on_requires_action: None,
        mandate: None,
        mandate_data: None,
        off_session: None,
        on_behalf_of: None,
        payment_method: None,
        payment_method_data: None,
        payment_method_options: Some(stripe::CreatePaymentIntentPaymentMethodOptions {
            card: Some(stripe::CreatePaymentIntentPaymentMethodOptionsCard {
                request_three_d_secure: Some(stripe::CreatePaymentIntentPaymentMethodOptionsCardRequestThreeDSecure::Automatic),
                ..Default::default()
            }),
            ..Default::default()
        }),
        receipt_email: None,
        return_url: None,
        shipping: None,
        transfer_data: None,
        transfer_group: None,
        application_fee_amount: None,
        use_stripe_sdk: None,
        expand: &[],
        payment_method_configuration: None,
        radar_options: None,
    };

    let intent = PaymentIntent::create(&client, params)
        .await
        .map_err(DonationError::StripeError)?;

    info!("Payment intent created successfully");
    
    let amount_dollars = request.amount / 100;
    
    let (delegate_certificate, _) = get_delegate(amount_dollars as u64).map_err(|e| {
        error!("Error getting delegate: {:?}", e);
        DonationError::OtherError("Error getting delegate".to_string())
    })?;
    
    match intent.client_secret {
        Some(secret) => {
            Ok(Json(DonationResponse {
                client_secret: secret,
                payment_intent_id: intent.id.to_string(),
                delegate_certificate_base64: delegate_certificate.to_base64().unwrap(),
            }))
        },
        None => {
            error!("Client secret is missing from the PaymentIntent");
            Err(DonationError::OtherError("Client secret is missing".to_string()))
        }
    }
}

#[derive(Deserialize)]
#[derive(Debug)]
pub struct UpdateDonationRequest {
    pub payment_intent_id: String,
    pub amount: i64,
}

async fn update_donation(
    Json(request): Json<UpdateDonationRequest>,
) -> Result<Json<DonationResponse>, DonationError> {
    info!("Received update-donation request: {:?}", request);

    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(DonationError::EnvError)?;
    let client = Client::new(&secret_key);

    let payment_intent_id = PaymentIntentId::from_str(&request.payment_intent_id).map_err(|_| DonationError::InvalidCurrency)?;
    let params = stripe::UpdatePaymentIntent {
        amount: Some(request.amount),
        ..Default::default()
    };

    let updated_intent = stripe::PaymentIntent::update(&client, &payment_intent_id, params)
        .await
        .map_err(DonationError::StripeError)?;

    info!("Payment intent updated successfully");

    let amount_dollars = request.amount / 100;
    
    let (delegate_certificate, _) = get_delegate(amount_dollars as u64).map_err(|e| {
        error!("Error getting delegate: {:?}", e);
        DonationError::OtherError("Error getting delegate".to_string())
    })?;
    
    Ok(Json(DonationResponse {
        client_secret: updated_intent.client_secret.unwrap_or_default(),
        payment_intent_id: updated_intent.id.to_string(),
        delegate_certificate_base64: delegate_certificate.to_base64().unwrap(),
    }))
}

async fn check_payment_status_route(
    Path(payment_intent_id): Path<String>,
) -> Result<StatusCode, DonationError> {
    info!("Received check-payment-status request for PaymentIntent ID: {}", payment_intent_id);

    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(DonationError::EnvError)?;
    let client = Client::new(&secret_key);

    let payment_intent_id = PaymentIntentId::from_str(&payment_intent_id).map_err(|_| DonationError::InvalidCurrency)?;

    let intent = stripe::PaymentIntent::retrieve(&client, &payment_intent_id, &[]).await.map_err(DonationError::StripeError)?;

    if intent.status == stripe::PaymentIntentStatus::Succeeded {
        info!("Payment intent succeeded");
        Ok(StatusCode::OK)
    } else {
        error!("Payment intent not successful: {:?}", intent.status);
        Err(DonationError::OtherError("Payment not successful".to_string()))
    }
}

pub fn get_routes() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/message", get(get_message))
        .route("/sign-certificate", post(sign_certificate_route))
        .route("/create-donation", post(create_donation))
        .route("/update-donation", post(update_donation))
        .route("/check-payment-status/:payment_intent_id", get(check_payment_status_route))
}
