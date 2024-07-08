#[macro_use]
extern crate rocket;

extern crate dotenv;
mod routes;
mod stripe_handler;

use rocket::fairing::AdHoc;
use rocket::shield::{Shield, XssFilter, Referrer};
use rocket::Request;
use env_logger::Env;
use dotenv::dotenv;

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}

#[catch(500)]
fn internal_error() -> &'static str {
    "Internal server error. Please try again later."
}

#[launch]
fn rocket() -> _ {
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    rocket::build()
        .attach(routes::CORS)
        .attach(routes::RequestTimer)
        .attach(AdHoc::on_response("Powered-By Header", |_, res| Box::pin(async move {
            res.set_raw_header("X-Powered-By", "Freenet Rocket API");
        })))
        .attach(Shield::new()
            .enable(XssFilter::EnableBlock)
            .enable(Referrer::NoReferrer)
            .enable(Referrer::StrictOriginWhenCrossOrigin))
        .attach(AdHoc::on_response("Content-Type-Options Header", |_, res| Box::pin(async move {
            res.set_raw_header("X-Content-Type-Options", "nosniff");
        })))
        .mount("/", routes::routes())
        .register("/", catchers![not_found, internal_error])
}
