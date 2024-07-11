use rocket::{launch, catchers};
use rocket::fairing::AdHoc;
use rocket::http::Header;
use rocket::{Request, catch};
use env_logger::Env;
use dotenv::dotenv;
use log::LevelFilter;
use chrono;

mod routes;
mod stripe_handler;

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
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            writeln!(buf,
                "{} [{}] {} - {}:{}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0)
            )
        })
        .filter(None, LevelFilter::Info)
        .init();

    log::info!("Starting Freenet Certified Donation API");
    match dotenv() {
        Ok(path) => log::info!(".env file loaded successfully from: {:?}", path),
        Err(e) => log::error!("Failed to load .env file: {}", e),
    }
    rocket::build()
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
        .mount("/", routes::routes())
        .register("/", catchers![not_found, internal_error])
}
