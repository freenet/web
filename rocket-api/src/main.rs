#[macro_use] extern crate rocket;

mod routes;
mod stripe_handler;

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes::routes())
}
