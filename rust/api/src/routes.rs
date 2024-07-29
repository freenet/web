use std::collections::HashMap;
use std::str::FromStr;
use std::time::Instant;

use log::{debug, error, info};
use rocket::{Request, Response, Route};
use rocket::{get, options, post, routes};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Header, Status};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use rocket::http::Status;
use rocket::serde::json::Json;

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    status: u16,
}
use stripe::{Client, Currency, PaymentIntent, PaymentIntentId};
use crate::delegates::get_delegate;
use crate::handle_sign_cert::{CertificateError, sign_certificate, SignCertificateRequest, SignCertificateResponse};

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let allowed_origins = ["http://localhost:1313", "https://freenet.org"];
        let origin = request.headers().get_one("Origin").unwrap_or("");
        
        if allowed_origins.contains(&origin) {
            response.set_header(Header::new("Access-Control-Allow-Origin", origin));
            response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
            response.set_header(Header::new("Access-Control-Allow-Headers", "Content-Type, Authorization"));
            response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
            response.set_header(Header::new("Access-Control-Max-Age", "86400"));
        }
    }
}

pub struct RequestTimer;

#[rocket::async_trait]
impl Fairing for RequestTimer {
    fn info(&self) -> Info {
        Info {
            name: "Request Timer",
            kind: Kind::Request | Kind::Response
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut rocket::Data<'_>) {
        request.local_cache(|| Instant::now());
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, _: &mut Response<'r>) {
        let start_time = request.local_cache(|| Instant::now());
        let duration = start_time.elapsed();
        debug!("Request to {} took {}ms", request.uri(), duration.as_millis());
    }
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
    pub delegate_base64 : String,
}

#[get("/")]
fn index() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "Hello, world!"
    }))
}

#[get("/message")]
fn get_message() -> Json<Message> {
    Json(Message {
        content: String::from("Welcome to the Freenet API! This message confirms that the API is functioning correctly."),
    })
}

#[post("/sign-certificate", data = "<request>")]
pub async fn sign_certificate_route(request: Json<SignCertificateRequest>) -> Result<Json<SignCertificateResponse>, Json<ErrorResponse>> {
    info!("Received sign-certificate request: {:?}", request);
    match sign_certificate(request.into_inner()).await {
        Ok(response) => {
            info!("Certificate signed successfully");
            Ok(Json(response))
        },
        Err(e) => {
            error!("Error signing certificate: {:?}", e);
            match e {
                CertificateError::PaymentNotSuccessful => {
                    Err(Json(ErrorResponse {
                        error: "Payment not successful. Please check your payment details and try again.".to_string(),
                        status: Status::BadRequest.code,
                    }))
                },
                CertificateError::CertificateAlreadySigned => {
                    Err(Json(ErrorResponse {
                        error: "Certificate has already been signed for this payment.".to_string(),
                        status: Status::Conflict.code,
                    }))
                },
                CertificateError::KeyError(msg) => {
                    Err(Json(ErrorResponse {
                        error: format!("Key error: {}", msg),
                        status: Status::InternalServerError.code,
                    }))
                },
                _ => {
                    Err(Json(ErrorResponse {
                        error: "An unexpected error occurred. Please try again later.".to_string(),
                        status: Status::InternalServerError.code,
                    }))
                }
            }
        },
    }
}

#[options("/sign-certificate")]
pub fn options_sign_certificate() -> Status {
    Status::Ok
}

#[options("/create-donation")]
pub fn options_create_donation() -> Status {
    Status::Ok
}

#[options("/create-payment-intent")]
pub fn options_create_payment_intent() -> Status {
    Status::Ok
}

#[derive(Debug)]
pub enum DonationError {
    InvalidCurrency,
    StripeError(stripe::StripeError),
    EnvError(std::env::VarError),
    OtherError(String),
}

impl<'r> rocket::response::Responder<'r, 'static> for DonationError {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        match self {
            DonationError::InvalidCurrency => Err(Status::BadRequest),
            DonationError::StripeError(e) => {
                eprintln!("Stripe error: {:?}", e);
                Err(Status::InternalServerError)
            },
            DonationError::EnvError(e) => {
                eprintln!("Environment variable error: {:?}", e);
                Err(Status::InternalServerError)
            },
            DonationError::OtherError(e) => {
                eprintln!("Other error: {:?}", e);
                Err(Status::InternalServerError)
            },
        }
    }
}

#[post("/create-donation", data = "<request>")]
pub async fn create_donation(request: Json<DonationRequest>) -> Result<Json<DonationResponse>, DonationError> {
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
        automatic_payment_methods: Some(stripe::CreatePaymentIntentAutomaticPaymentMethods {
            enabled: true,
            allow_redirects: None,
        }),
        metadata: Some(metadata),
        description: Some("Freenet Donation"),
        statement_descriptor: Some("Freenet Donation"),
        statement_descriptor_suffix: Some("Thank You"),
        capture_method: None,
        confirm: None,
        confirmation_method: None,
        customer: None,
        error_on_requires_action: None,
        mandate: None,
        mandate_data: None,
        off_session: None,
        on_behalf_of: None,
        payment_method: None,
        payment_method_data: None,
        payment_method_options: None,
        payment_method_types: None,
        receipt_email: None,
        return_url: None,
        setup_future_usage: None,
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
    
    let (delegate_certificate, _) =  get_delegate(request.amount as u64).map_err(|e| DonationError::OtherError(e.to_string()))?;
    
    match intent.client_secret {
        Some(secret) => {
            Ok(Json(DonationResponse {
                client_secret: secret,
                payment_intent_id: intent.id.to_string(),
                delegate_base64: delegate_certificate.to_base64(),
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

#[post("/update-donation", data = "<request>")]
pub async fn update_donation(request: Json<UpdateDonationRequest>) -> Result<Status, DonationError> {
    info!("Received update-donation request: {:?}", request);

    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(DonationError::EnvError)?;
    let client = Client::new(&secret_key);

    let payment_intent_id = PaymentIntentId::from_str(&request.payment_intent_id).map_err(|_| DonationError::InvalidCurrency)?;
    let params = stripe::UpdatePaymentIntent {
        amount: Some(request.amount),
        ..Default::default()
    };

    stripe::PaymentIntent::update(&client, &payment_intent_id, params)
        .await
        .map_err(DonationError::StripeError)?;

    info!("Payment intent updated successfully");
    Ok(Status::Ok)
}

#[get("/check-payment-status/<payment_intent_id>")]
pub async fn check_payment_status_route(payment_intent_id: String) -> Result<Status, DonationError> {
    info!("Received check-payment-status request for PaymentIntent ID: {}", payment_intent_id);

    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(DonationError::EnvError)?;
    let client = Client::new(&secret_key);

    let payment_intent_id = PaymentIntentId::from_str(&payment_intent_id).map_err(|_| DonationError::InvalidCurrency)?;

    let intent = stripe::PaymentIntent::retrieve(&client, &payment_intent_id, &[]).await.map_err(DonationError::StripeError)?;

    if intent.status == stripe::PaymentIntentStatus::Succeeded {
        info!("Payment intent succeeded");
        Ok(Status::Ok)
    } else {
        error!("Payment intent not successful: {:?}", intent.status);
        Err(DonationError::OtherError("Payment not successful".to_string()))
    }
}
pub fn get_routes() -> Vec<Route> {
    routes![
        index,
        get_message,
        sign_certificate_route,
        options_sign_certificate,
        create_donation,
        options_create_donation,
        check_payment_status_route,
        update_donation,
        options_create_payment_intent
    ]
}
