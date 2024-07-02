#[macro_use] extern crate rocket;

use dotenv::dotenv;
mod routes;
mod stripe_handler;

use crate::routes::CORS;

#[launch]
fn rocket() -> _ {
    dotenv().ok();
    rocket::build()
        .attach(CORS)
        .mount("/", routes::routes())
}
