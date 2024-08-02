use std::{env, time::SystemTime, fs};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use clap::{Arg, Command};
use dotenv::dotenv;
use log::{error, info, LevelFilter};
use axum::{
    routing::{get, post},
    Router, Server,
    http::{StatusCode, HeaderMap, HeaderValue},
    response::IntoResponse,
    extract::State,
};
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;

mod routes;
mod handle_sign_cert;
mod delegates;
mod errors;

pub static DELEGATE_DIR: &str = "DELEGATE_DIR";

struct TlsConfig {
    cert: String,
    key: String,
    last_modified: SystemTime,
}

impl TlsConfig {
    fn new(cert: String, key: String) -> Self {
        let last_modified = SystemTime::now();
        Self { cert, key, last_modified }
    }

    fn update_if_changed(&mut self) -> bool {
        let cert_modified = fs::metadata(&self.cert).and_then(|m| m.modified()).ok();
        let key_modified = fs::metadata(&self.key).and_then(|m| m.modified()).ok();

        if let (Some(cert_time), Some(key_time)) = (cert_modified, key_modified) {
            let max_time = cert_time.max(key_time);
            if max_time > self.last_modified {
                self.last_modified = max_time;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Sorry, this is not a valid path.")
}

async fn internal_error() -> impl IntoResponse {
    (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error. Please try again later.")
}

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() {
    let matches = Command::new("Freenet Certified Donation API")
        .arg(Arg::new("delegate-dir")
            .long("delegate-dir")
            .value_name("DIR")
            .help("Sets the delegate directory")
            .required(true))
        .arg(Arg::new("tls-cert")
            .long("tls-cert")
            .value_name("FILE")
            .help("Path to TLS certificate file"))
        .arg(Arg::new("tls-key")
            .long("tls-key")
            .value_name("FILE")
            .help("Path to TLS private key file"))
        .get_matches();

    let delegate_dir = matches.get_one::<String>("delegate-dir").unwrap();
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
    
    let tls_config = if let (Some(tls_cert), Some(tls_key)) = (matches.get_one::<String>("tls-cert"), matches.get_one::<String>("tls-key")) {
        info!("TLS certificate and key provided. Starting in HTTPS mode.");
        Some(Arc::new(Mutex::new(TlsConfig::new(tls_cert.to_string(), tls_key.to_string()))))
    } else {
        info!("No TLS certificate and key provided. Starting in HTTP mode.");
        None
    };

    let app = Router::new()
        .route("/health", get(health))
        .merge(routes::get_routes())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .fallback(not_found)
        .with_state(tls_config.clone());

    if let Some(tls_config) = tls_config.clone() {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // Check every hour
            loop {
                interval.tick().await;
                let mut config = tls_config.lock().unwrap();
                if config.update_if_changed() {
                    info!("TLS certificate or key has been updated. Reloading configuration.");
                    // In a real implementation, you would need to signal the server to reload its TLS config here.
                    // This might involve restarting the server or using a more sophisticated hot-reload mechanism.
                }
            }
        });
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Listening on {}", addr);
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
