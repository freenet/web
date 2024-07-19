use rocket::{launch, catchers};
use rocket::fairing::AdHoc;
use rocket::http::Header;
use rocket::{Request, catch};
use dotenv::dotenv;
use log::{LevelFilter, info, error};

mod routes;
mod stripe_handler;

pub static DELEGATE_DIR: &str = "GHOSTKEY_DELEGATE_DIR";

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
        .mount("/", routes::get_routes())
        .register("/", catchers![not_found, internal_error])
}
