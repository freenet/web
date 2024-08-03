use std::{env, path::PathBuf, sync::Arc};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use clap::{Arg, Command, value_parser};
use dotenv::dotenv;
use log::{error, info, LevelFilter};
use tokio::sync::Mutex;
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

async fn serve_http01_challenge(
    challenge_dir: Arc<Mutex<Option<PathBuf>>>,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let challenge_dir = challenge_dir.lock().await;
    
    if let Some(dir) = &*challenge_dir {
        let file_path = dir.join(path);
        if file_path.is_file() {
            match tokio::fs::read_to_string(file_path).await {
                Ok(content) => (StatusCode::OK, content),
                Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read challenge file".to_string()),
            }
        } else {
            (StatusCode::NOT_FOUND, "Challenge file not found".to_string())
        }
    } else {
        (StatusCode::NOT_FOUND, "Challenge directory not configured".to_string())
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
        .arg(Arg::new("port")
            .long("port")
            .value_name("PORT")
            .help("Sets the port to listen on")
            .value_parser(value_parser!(u16)))
        .arg(Arg::new("challenge-dir")
            .long("challenge-dir")
            .value_name("DIR")
            .help("Directory for HTTP-01 challenge tokens"))
        .get_matches();

    let delegate_dir = matches.get_one::<String>("delegate-dir").unwrap();
    let user_port = matches.get_one::<u16>("port");
    let challenge_dir = matches.get_one::<String>("challenge-dir").map(PathBuf::from);
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
    
    let challenge_dir = Arc::new(Mutex::new(challenge_dir));

    let app = Router::new()
        .route("/health", get(health))
        .merge(routes::get_routes())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .fallback(not_found);

    let challenge_dir_clone = challenge_dir.clone();
    let challenge_app = Router::new()
        .fallback(move |uri| serve_http01_challenge(challenge_dir_clone.clone(), uri));

    let (is_https, default_port) = if matches.get_one::<String>("tls-cert").is_some() && matches.get_one::<String>("tls-key").is_some() {
        (true, 443)
    } else {
        (false, 8000)
    };

    let port = user_port.copied().unwrap_or(default_port);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
    info!("Listening on {}", addr);

    let main_server = async {
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
    };

    if challenge_dir.lock().await.is_some() {
        let http_challenge_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 80);
        info!("Starting HTTP-01 challenge server on {}", http_challenge_addr);
        let challenge_listener = tokio::net::TcpListener::bind(http_challenge_addr).await.unwrap();
        let challenge_server = tokio::task::spawn(async move {
            axum::serve(challenge_listener, challenge_app).await.unwrap();
        });

        tokio::select! {
            _ = main_server => {},
            _ = challenge_server => {},
        }
    } else {
        main_server.await;
    }
}
