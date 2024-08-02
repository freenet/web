use std::{env, time::SystemTime};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use clap::{Arg, Command};
use dotenv::dotenv;
use log::{error, info, LevelFilter};
use axum::{
    routing::get,
    Router,
    http::StatusCode,
    response::IntoResponse,
};
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;
use tokio::net::TcpListener;
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};
use tokio_rustls::{TlsAcceptor, TlsListener};
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};
use tokio_rustls::{TlsAcceptor, TlsListener};

mod routes;
mod handle_sign_cert;
mod delegates;
mod errors;

pub static DELEGATE_DIR: &str = "DELEGATE_DIR";

struct TlsConfig {
    cert: Vec<u8>,
    key: Vec<u8>,
    last_modified: SystemTime,
}

impl TlsConfig {
    fn new(cert: Vec<u8>, key: Vec<u8>) -> Self {
        let last_modified = SystemTime::now();
        Self { cert, key, last_modified }
    }
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Sorry, this is not a valid path.")
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
        let cert = tokio::fs::read(tls_cert).await.expect("Failed to read TLS certificate");
        let key = tokio::fs::read(tls_key).await.expect("Failed to read TLS key");
        Some(Arc::new(Mutex::new(TlsConfig::new(cert, key))))
    } else {
        info!("No TLS certificate and key provided. Starting in HTTP mode.");
        None
    };

    let app = Router::new()
        .route("/health", get(health))
        .merge(routes::get_routes())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .fallback(not_found);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("Listening on {}", addr);
    
    if let Some(tls_config) = tls_config {
        let config = tls_config.lock().unwrap();
        let cert = Certificate(config.cert.clone());
        let key = PrivateKey(config.key.clone());
        
        let server_config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key)
            .expect("Failed to create TLS server config");
        
        let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
        
        info!("Starting server with TLS (HTTPS)");
        let listener = TcpListener::bind(addr).await.unwrap();
        let incoming_tls = TlsListener::new(tls_acceptor, listener);
        
        axum::serve(incoming_tls, app).await.unwrap();
    } else {
        info!("Starting server without TLS (HTTP)");
        let listener = TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}
