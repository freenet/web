use crate::stripe_handler::{sign_certificate, SignCertificateRequest, SignCertificateResponse};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{Header, Status};
use rocket::serde::json::Json;
use rocket::{Data, Request, Response};
use rocket::form::Form;
use rocket::fs::TempFile;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use stripe::{Client, Currency};
use log::{info, error};
use std::path::Path;
use std::fs;
use rocket::data::ToByteUnit;

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
pub struct DonationResponse {
    pub client_secret: String,
}

#[derive(FromForm)]
pub struct UploadForm<'f> {
    #[field(validate = ext(["pdf", "doc", "docx", "jpg", "jpeg"]))]
    file: TempFile<'f>,
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
    info!("Received sign-certificate request: {:?}", request);
    match sign_certificate(request.into_inner()).await {
        Ok(response) => {
            info!("Certificate signed successfully");
            Ok(Json(response))
        },
        Err(e) => {
            error!("Error signing certificate: {}", e);
            Err((Status::InternalServerError, format!("Error signing certificate: {}", e)))
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
    StripeError(Box<stripe::StripeError>),
    EnvError(Box<std::env::VarError>),
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
    
    let secret_key = std::env::var("STRIPE_SECRET_KEY").map_err(|e| {
        error!("Failed to get STRIPE_SECRET_KEY: {:?}", e);
        DonationError::EnvError(Box::new(e))
    })?;
    let client = Client::new(secret_key);

    let currency = match request.currency.as_str() {
        "usd" => Currency::USD,
        "eur" => Currency::EUR,
        "gbp" => Currency::GBP,
        _ => {
            error!("Invalid currency: {}", request.currency);
            return Err(DonationError::InvalidCurrency);
        }
    };

    let mut params = stripe::CreatePaymentIntent::new(request.amount, currency);
    params.payment_method_types = Some(vec!["card".to_string()]);

    let intent = stripe::PaymentIntent::create(&client, params)
        .await
        .map_err(|e| {
            error!("Stripe error: {:?}", e);
            DonationError::StripeError(Box::new(e))
        })?;

    info!("Payment intent created successfully");
    Ok(Json(DonationResponse {
        client_secret: intent.client_secret.unwrap_or_default(),
    }))
}

#[post("/upload", data = "<form>")]
pub async fn upload(mut form: Form<UploadForm<'_>>) -> Result<String, Status> {
    let file = &mut form.file;
    let file_name = file.name().unwrap_or("unknown").to_string();
    let upload_dir = Path::new("uploads");

    if !upload_dir.exists() {
        fs::create_dir(upload_dir).map_err(|_| Status::InternalServerError)?;
    }

    let file_path = upload_dir.join(&file_name);
    file.persist_to(&file_path).await.map_err(|_| Status::InternalServerError)?;

    Ok(format!("File '{}' uploaded successfully", file_name))
}

#[options("/upload")]
pub fn options_upload() -> Status {
    Status::Ok
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        index,
        get_message,
        sign_certificate_route,
        options_sign_certificate,
        create_donation,
        options_create_donation,
        upload,
        options_upload
    ]
}
