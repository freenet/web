use std::{env, fs};
use std::sync::Arc;
use std::net::SocketAddr;

use clap::{Arg, Command};
use dotenv::dotenv;
use log::{error, info, LevelFilter};
use tokio_rustls::rustls::{Certificate, PrivateKey};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use axum::{
    routing::get,
    Router,
    http::StatusCode,
    response::IntoResponse,
    Server,
};
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;

mod routes;
mod handle_sign_cert;
mod delegates;
mod errors;

pub static DELEGATE_DIR: &str = "DELEGATE_DIR";

// TLS configuration struct
struct TlsConfig {
    cert: String,
    key: String,
}

impl TlsConfig {
    fn new(cert: String, key: String) -> Self {
        Self { cert, key }
    }
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Sorry, this is not a valid path.")
}

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        Some(TlsConfig::new(tls_cert.to_string(), tls_key.to_string()))
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
        let server_config = load_tls_config(&tls_config)?;
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum_tls::bind_rustls(listener, acceptor)
            .serve(app.into_make_service())
            .await?;
    } else {
        Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
    }
    Ok(())
}

fn load_tls_config(tls_config: &TlsConfig) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let mut cert_file = std::io::BufReader::new(fs::File::open(&tls_config.cert)?);
    let mut key_file = std::io::BufReader::new(fs::File::open(&tls_config.key)?);
    
    let cert_chain = rustls_pemfile::certs(&mut cert_file)?
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut key_file)?;

    if keys.is_empty() {
        return Err("No PKCS8 private keys found in key file".into());
    }

    let server_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(cert_chain, PrivateKey(keys.remove(0)))?;

    Ok(server_config)
}
