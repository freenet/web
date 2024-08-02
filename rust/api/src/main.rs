use std::{env, fs, sync::Arc, net::SocketAddr, path::PathBuf};
use clap::{Arg, Command};
use dotenv::dotenv;
use log::{error, info, LevelFilter};
use axum::{
    routing::get,
    Router,
    http::StatusCode,
    response::IntoResponse,
    Server,
};
use tower_http::trace::TraceLayer;
use tower_http::cors::CorsLayer;
use tokio::net::TcpListener;
use axum_server::tls_rustls::RustlsConfig;

mod routes;
mod handle_sign_cert;
mod delegates;
mod errors;

pub static DELEGATE_DIR: &str = "DELEGATE_DIR";

// TLS configuration struct
struct TlsConfig {
    cert: PathBuf,
    key: PathBuf,
}

impl TlsConfig {
    fn new(cert: PathBuf, key: PathBuf) -> Self {
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
        Some(TlsConfig::new(PathBuf::from(tls_cert), PathBuf::from(tls_key)))
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
        let config = RustlsConfig::from_pem_file(
            tls_config.cert,
            tls_config.key,
        ).await?;
        let listener = TcpListener::bind(addr).await?;
        axum_server::from_tcp_rustls(listener, config)
            .serve(app.into_make_service())
            .await?;
    } else {
        Server::bind(&addr)
            .serve(app.into_make_service())
            .await?;
    }
    Ok(())
}

