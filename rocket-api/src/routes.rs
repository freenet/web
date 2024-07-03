use rocket::http::Header;
use rocket::{Request, Response, Data};
use rocket::fairing::{Fairing, Info, Kind};
use serde::{Serialize, Deserialize};
use rocket::serde::json::Json;
use crate::stripe_handler::{SignCertificateRequest, SignCertificateResponse, sign_certificate};
use rocket::http::Status;
use std::time::Instant;

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

pub fn routes() -> Vec<rocket::Route> {
    routes![index, get_message, sign_certificate_route, options_sign_certificate]
}
