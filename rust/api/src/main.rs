use std::{env, fs, path::PathBuf, sync::Arc};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

use clap::{Arg, Command, value_parser};
use dotenv::dotenv;
use ed25519_dalek::{SigningKey, VerifyingKey};
use log::{error, info, warn, LevelFilter};
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

use crate::routes::InviteState;

mod routes;
mod handle_sign_cert;
mod delegates;
mod errors;
mod invite;
mod rate_limit;

pub static DELEGATE_DIR: &str = "DELEGATE_DIR";

async fn serve_http01_challenge(
    challenge_dir: Arc<Mutex<Option<PathBuf>>>,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/').trim_start_matches(".well-known/acme-challenge/");
    let challenge_dir = challenge_dir.lock().await;
    
    if let Some(dir) = &*challenge_dir {
        let file_path = dir.join(path);
        if file_path.is_file() {
            match tokio::fs::read_to_string(&file_path).await {
                Ok(content) => (StatusCode::OK, content),
                Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read challenge file".to_string()),
            }
        } else {
            let error_message = format!("Challenge file not found at path: {}", file_path.display());
            (StatusCode::NOT_FOUND, error_message)
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

/// Load invite configuration from CLI arguments
/// Returns None if not all required configuration is present
fn load_invite_config(matches: &clap::ArgMatches) -> Option<InviteState> {
    let signing_key_path = matches.get_one::<String>("room-signing-key")?;
    let owner_vk_str = matches.get_one::<String>("room-owner-vk")?;
    let room_name = matches.get_one::<String>("room-name")?.clone();
    let rate_limit_file = PathBuf::from(
        matches.get_one::<String>("rate-limit-file")
            .map(|s| s.as_str())
            .unwrap_or("/var/lib/gkapi/invite_rate_limits.json")
    );

    // Load signing key from file (32 bytes raw)
    let signing_key_bytes = match fs::read(signing_key_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read room signing key from {}: {}", signing_key_path, e);
            return None;
        }
    };

    if signing_key_bytes.len() != 32 {
        error!("Room signing key must be exactly 32 bytes, got {}", signing_key_bytes.len());
        return None;
    }

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&signing_key_bytes);
    let inviter_signing_key = SigningKey::from_bytes(&key_array);

    // Parse owner verifying key from base58
    let owner_vk_bytes = match bs58::decode(owner_vk_str).into_vec() {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to decode room owner verifying key from base58: {}", e);
            return None;
        }
    };

    if owner_vk_bytes.len() != 32 {
        error!("Room owner verifying key must be exactly 32 bytes, got {}", owner_vk_bytes.len());
        return None;
    }

    let mut vk_array = [0u8; 32];
    vk_array.copy_from_slice(&owner_vk_bytes);
    let room_owner_vk = match VerifyingKey::from_bytes(&vk_array) {
        Ok(vk) => vk,
        Err(e) => {
            error!("Failed to parse room owner verifying key: {}", e);
            return None;
        }
    };

    Some(InviteState::new(
        rate_limit_file,
        room_owner_vk,
        inviter_signing_key,
        room_name,
    ))
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
        // River room invite configuration
        .arg(Arg::new("room-signing-key")
            .long("room-signing-key")
            .value_name("FILE")
            .env("ROOM_SIGNING_KEY_FILE")
            .help("Path to room member's signing key (32 bytes raw)"))
        .arg(Arg::new("room-owner-vk")
            .long("room-owner-vk")
            .value_name("KEY")
            .env("ROOM_OWNER_VK")
            .help("Room owner's verifying key (base58 encoded)"))
        .arg(Arg::new("room-name")
            .long("room-name")
            .value_name("NAME")
            .env("ROOM_NAME")
            .default_value("Freenet Chat")
            .help("Display name of the room"))
        .arg(Arg::new("rate-limit-file")
            .long("rate-limit-file")
            .value_name("FILE")
            .env("RATE_LIMIT_FILE")
            .default_value("/var/lib/gkapi/invite_rate_limits.json")
            .help("Path to rate limit JSON file"))
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

    // Load invite configuration (optional)
    let invite_state = load_invite_config(&matches);

    let mut app = Router::new()
        .route("/health", get(health))
        .merge(routes::get_routes());

    // Add invite routes if configured
    if let Some(state) = invite_state {
        info!("River room invite endpoint enabled for room: {}", state.room_name);
        app = app.merge(routes::get_invite_routes(state));
    } else {
        warn!("River room invite endpoint not configured. Set ROOM_SIGNING_KEY_FILE and ROOM_OWNER_VK to enable.");
    }

    let app = app
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
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        } else {
            info!("No TLS certificate and key provided. Starting in HTTP mode.");
            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
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
