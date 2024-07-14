use crate::stripe_handler::{sign_certificate, SignCertificateRequest, SignCertificateResponse, DelegateInfo, CertificateError};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Header, Status};
use rocket::serde::json::Json;
use rocket::{Request, Response, Route};
use rocket::{get, post, options, routes};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use stripe::{Client, Currency, PaymentIntent, PaymentIntentId};
use std::str::FromStr;
use log::{info, error};
use serde_json::json;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "http://localhost:1313"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
        response.set_header(Header::new("Access-Control-Max-Age", "86400"));
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
        println!("Request to {} took {}ms", request.uri(), duration.as_millis());
    }
}

#[derive(Serialize, Deserialize)]
struct Message {
    content: String,
}

#[derive(Deserialize, Debug)]
pub struct DonationRequest {
    pub currency: String,
}

#[derive(Serialize)]
pub struct DonationResponse {
    pub client_secret: String,
    pub payment_intent_id: String,
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
pub async fn sign_certificate_route(request: Json<SignCertificateRequest>) -> Result<Json<SignCertificateResponse>, Status> {
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
                    error!("Payment not successful. Please check the Stripe dashboard for more details.");
                    Err(Status::BadRequest)
                },
                CertificateError::CertificateAlreadySigned => {
                    info!("Certificate already signed");
                    Ok(Json(SignCertificateResponse {
                        blind_signature: String::new(),
                        delegate_info: DelegateInfo::default(),
                    }))
                },
                _ => {
                    error!("Unexpected error: {:?}", e);
                    Err(Status::InternalServerError)
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

#[derive(Debug)]
pub enum DonationError {
    InvalidCurrency,
    StripeError(stripe::StripeError),
    EnvError(std::env::VarError),
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
        }
    }
}

#[post("/create-donation", data = "<request>")]
pub async fn create_donation(request: Json<DonationRequest>) -> Result<Json<DonationResponse>, DonationError> {
    info!("Received create-donation request: {:?}", request);
    
    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(DonationError::EnvError)?;
    let client = Client::new(&secret_key);

    let currency = match request.currency.as_str() {
        "usd" => Currency::USD,
        _ => {
            error!("Invalid currency: {}", request.currency);
            return Err(DonationError::InvalidCurrency);
        }
    };

    let params = stripe::CreatePaymentIntent {
        amount: 500, // Placeholder amount (e.g., $5.00)
        currency,
        payment_method_types: Some(vec!["card".to_string()]),
        capture_method: Some(stripe::PaymentIntentCaptureMethod::Manual),
        // ..Default::default() // Remove this line
    };

    let intent = stripe::PaymentIntent::create(&client, params)
        .await
        .map_err(DonationError::StripeError)?;

    info!("Payment intent created successfully");
    match intent.client_secret {
        Some(secret) => {
            Ok(Json(DonationResponse {
                client_secret: secret,
                payment_intent_id: intent.id.to_string(),
            }))
        },
        None => {
            error!("Client secret is missing from the PaymentIntent");
            Err(DonationError::StripeError(stripe::StripeError::InvalidRequestError(stripe::InvalidRequestError {
                message: Some("Client secret is missing".to_string()),
                http_status: 0,
                error_type: stripe::ErrorType::InvalidRequest,
                code: None,
                charge: None,
                decline_code: None,
            })))
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
pub async fn check_payment_status(payment_intent_id: String) -> Result<Json<serde_json::Value>, Status> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(|_| Status::InternalServerError)?;
    let client = Client::new(&secret_key);

    let payment_intent_id = PaymentIntentId::from_str(&payment_intent_id).map_err(|_| Status::BadRequest)?;
    let pi = PaymentIntent::retrieve(&client, &payment_intent_id, &[])
        .await
        .map_err(|e| {
            error!("Error retrieving PaymentIntent: {:?}", e);
            Status::InternalServerError
        })?;

    Ok(Json(json!({
        "status": pi.status.to_string()
    })))
}

pub fn routes() -> Vec<Route> {
    routes![
        index,
        get_message,
        sign_certificate_route,
        options_sign_certificate,
        create_donation,
        options_create_donation,
        check_payment_status,
        update_donation
    ]
}
