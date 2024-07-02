use rocket::http::Header;
use rocket::{Request, Response, Data};
use rocket::fairing::{Fairing, Info, Kind};
use serde::{Serialize, Deserialize};
use rocket::serde::json::Json;
use crate::stripe_handler::{DonationRequest, DonationResponse, create_payment_intent};
use rocket::http::Status;
use std::time::Instant;
use crate::stripe_handler::verify_payment_intent;

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

#[post("/create-donation", data = "<donation>")]
pub async fn create_donation(donation: Json<DonationRequest>) -> Result<Json<DonationResponse>, (Status, String)> {
    match create_payment_intent(donation.into_inner()).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            eprintln!("Error creating donation: {}", e);
            Err((Status::InternalServerError, format!("Error creating donation: {}", e)))
        },
    }
}

#[options("/create-donation")]
pub fn options_create_donation() -> Status {
    Status::Ok
}

#[get("/verify-payment/<payment_intent_id>")]
pub async fn verify_payment(payment_intent_id: String) -> Result<Status, (Status, String)> {
    match verify_payment_intent(payment_intent_id).await {
        Ok(is_valid) => {
            if is_valid {
                Ok(Status::Ok)
            } else {
                Ok(Status::PaymentRequired)
            }
        },
        Err(e) => {
            eprintln!("Error verifying payment: {}", e);
            Err((Status::InternalServerError, format!("Error verifying payment: {}", e)))
        },
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index, get_message, create_donation, options_create_donation, verify_payment]
}
