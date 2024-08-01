use std::{env, time::SystemTime, fs};
use std::sync::{Arc, Mutex};

use clap::{Arg, Command};
use dotenv::dotenv;
use log::{error, info, LevelFilter};
use rocket::{catchers, get, launch, routes, Config};
use rocket::{catch, Request};
use rocket::fairing::AdHoc;
use rocket::http::Header;
use rocket::tokio::{self, time::{interval, Duration}};

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

    let config = rocket::Config::figment();
    let rocket = rocket::custom(config);

    if let Some(tls_config) = tls_config.clone() {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // Check every hour
            loop {
                interval.tick().await;
                let mut config = tls_config.lock().unwrap();
                if config.update_if_changed() {
                    info!("TLS certificate or key has been updated. Reloading configuration.");
                    // In a real implementation, you would need to signal Rocket to reload its TLS config here.
                    // This might involve restarting the server or using a more sophisticated hot-reload mechanism.
                }
            }
        });
    }

    rocket
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
        .attach(AdHoc::on_ignite("TLS Config", |rocket| async move {
            if let Some(tls_config) = tls_config {
                let config = tls_config.lock().unwrap();
                rocket.configure(Config::figment()
                    .merge(("tls.certs", &config.cert))
                    .merge(("tls.key", &config.key)))
            } else {
                rocket
            }
        }))
        .mount("/", routes::get_routes())
        .mount("/", routes![health])
        .register("/", catchers![not_found, internal_error])
}
