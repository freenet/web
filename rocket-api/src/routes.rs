use serde::{Serialize, Deserialize};
use rocket::serde::json::Json;
use crate::stripe_handler::{DonationRequest, DonationResponse, create_payment_intent};

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
        content: String::from("This is a message from the Rocket API!"),
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
