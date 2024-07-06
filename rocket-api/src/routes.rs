use crate::stripe_handler::{sign_certificate, SignCertificateRequest, SignCertificateResponse};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Header, Status};
use rocket::serde::json::Json;
use rocket::{Data, Request, Response};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use stripe::{Client, Currency};

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

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
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
    pub amount: i64,
    pub currency: String,
}

#[derive(Serialize)]
struct DonationResponse {
    client_secret: String,
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/message")]
fn get_message() -> Json<Message> {
    Json(Message {
        content: String::from("Welcome to the Freenet API! This message confirms that the API is functioning correctly."),
    })
}


#[post("/sign-certificate", data = "<request>")]
pub async fn sign_certificate_route(request: Json<SignCertificateRequest>) -> Result<Json<SignCertificateResponse>, (Status, String)> {
    match sign_certificate(request.into_inner()).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            eprintln!("Error signing certificate: {}", e);
            Err((Status::InternalServerError, format!("Error signing certificate: {}", e)))
        },
    }
}

#[options("/sign-certificate")]
pub fn options_sign_certificate() -> Status {
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
            DonationError::StripeError(_) => Err(Status::InternalServerError),
            DonationError::EnvError(_) => Err(Status::InternalServerError),
        }
    }
}

#[post("/create-donation", data = "<request>")]
pub async fn create_donation(request: Json<DonationRequest>) -> Result<Json<DonationResponse>, DonationError> {
    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(DonationError::EnvError)?;
    let client = Client::new(secret_key);

    let currency = match request.currency.as_str() {
        "usd" => Currency::USD,
        "eur" => Currency::EUR,
        "gbp" => Currency::GBP,
        _ => return Err(DonationError::InvalidCurrency),
    };

    let params = stripe::CreatePaymentIntent::new(request.amount, currency);
    let intent = stripe::PaymentIntent::create(&client, params)
        .await
        .map_err(DonationError::StripeError)?;

    Ok(Json(DonationResponse {
        client_secret: intent.client_secret.unwrap(),
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        index,
        get_message,
        sign_certificate_route,
        options_sign_certificate,
        create_donation
    ]
}
