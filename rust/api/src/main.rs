use std::env;

use clap::{Arg, Command};
use dotenv::dotenv;
use log::{error, info, LevelFilter};
use rocket::{catchers, get, launch, routes, Config};
use rocket::{catch, Request};
use rocket::fairing::AdHoc;
use rocket::http::Header;

mod routes;
mod handle_sign_cert;
mod delegates;
mod errors;

pub static DELEGATE_DIR: &str = "DELEGATE_DIR";

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}

#[catch(500)]
fn internal_error() -> &'static str {
    "Internal server error. Please try again later."
}

#[get("/health")]
fn health() -> &'static str {
    "OK"
}

#[launch]
fn rocket() -> _ {
    let matches = Command::new("Freenet Certified Donation API")
        .arg(Arg::new("delegate-dir")
            .long("delegate-dir")
            .value_name("DIR")
            .help("Sets the delegate directory")
            .required(true))
        .arg(Arg::new("tls-cert")
            .long("tls-cert")
            .value_name("FILE")
            .help("Path to TLS certificate file")
            .required(true))
        .arg(Arg::new("tls-key")
            .long("tls-key")
            .value_name("FILE")
            .help("Path to TLS private key file")
            .required(true))
        .get_matches();

    let delegate_dir = matches.get_one::<String>("delegate-dir").unwrap();
    let tls_cert = matches.get_one::<String>("tls-cert").unwrap();
    let tls_key = matches.get_one::<String>("tls-key").unwrap();

    env::set_var(DELEGATE_DIR, delegate_dir);

    env_logger::builder()
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .format_module_path(false)
        .format_target(false)
        .filter_level(LevelFilter::Debug)
        .init();

    info!("Starting Freenet Certified Donation API");
    match dotenv() {
        Ok(path) => info!(".env file loaded successfully from: {:?}", path),
        Err(e) => error!("Failed to load .env file: {}", e),
    }

    env::var("DELEGATE_DIR").expect("DELEGATE_DIR environment variable not set");
    
    let config = Config::figment()
        .merge(("tls.certs", tls_cert))
        .merge(("tls.key", tls_key));

    rocket::custom(config)
        .attach(routes::CORS)
        .attach(routes::RequestTimer)
        .attach(AdHoc::on_response("Powered-By Header", |_, res| Box::pin(async move {
            res.set_header(Header::new("X-Powered-By", "Freenet GhostKey API"));
        })))
        .attach(AdHoc::on_response("Security Headers", |_, res| Box::pin(async move {
            res.set_header(Header::new("X-XSS-Protection", "1; mode=block"));
            res.set_header(Header::new("X-Content-Type-Options", "nosniff"));
            res.set_header(Header::new("Referrer-Policy", "strict-origin-when-cross-origin"));
        })))
        .mount("/", routes::get_routes())
        .mount("/", routes![health])
        .register("/", catchers![not_found, internal_error])
}
