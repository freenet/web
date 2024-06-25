use rocket::serde::json::Json;
use serde::{Serialize, Deserialize};

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

pub fn routes() -> Vec<rocket::Route> {
    routes![index, get_message]
}
