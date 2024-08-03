use std::{env, path::PathBuf};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use clap::{Arg, Command, value_parser};
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
use axum_server::tls_rustls::RustlsConfig;

mod routes;
mod handle_sign_cert;
mod delegates;
mod errors;

pub static DELEGATE_DIR: &str = "DELEGATE_DIR";


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
        .arg(Arg::new("port")
            .long("port")
            .value_name("PORT")
            .help("Sets the port to listen on")
            .value_parser(value_parser!(u16)))
        .get_matches();

    let delegate_dir = matches.get_one::<String>("delegate-dir").unwrap();
    let user_port = matches.get_one::<u16>("port");
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
    
    let app = Router::new()
        .route("/health", get(health))
        .merge(routes::get_routes())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .fallback(not_found);

    let (is_https, default_port) = if matches.get_one::<String>("tls-cert").is_some() && matches.get_one::<String>("tls-key").is_some() {
        (true, 443)
    } else {
        (false, 8000)
    };

    let port = user_port.copied().unwrap_or(default_port);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
    info!("Listening on {}", addr);

    if is_https {
        info!("TLS certificate and key provided. Starting in HTTPS mode.");
        let tls_cert = matches.get_one::<String>("tls-cert").unwrap();
        let tls_key = matches.get_one::<String>("tls-key").unwrap();
        let tls_config = RustlsConfig::from_pem_file(PathBuf::from(tls_cert), PathBuf::from(tls_key)).await.unwrap();
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        info!("No TLS certificate and key provided. Starting in HTTP mode.");
        axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
    }
}
