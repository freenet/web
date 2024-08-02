use axum::{
    routing::get,
    Router, Server,
    http::StatusCode,
    response::IntoResponse,
};
use std::net::SocketAddr;
use tokio;

mod routes;
mod handle_sign_cert;
mod errors;

#[tokio::main]
async fn main() {
    let app = routes::get_routes();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
