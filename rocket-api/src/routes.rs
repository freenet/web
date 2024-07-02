use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use serde::{Serialize, Deserialize};
use rocket::serde::json::Json;
use crate::stripe_handler::{DonationRequest, DonationResponse, create_payment_intent};

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
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
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
pub async fn create_donation(donation: Json<DonationRequest>) -> Result<Json<DonationResponse>, String> {
    match create_payment_intent(donation.into_inner()).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err(format!("Error creating donation: {}", e)),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    routes![index, get_message, create_donation]
}
