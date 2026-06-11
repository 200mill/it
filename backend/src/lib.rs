use axum::{
    routing::{get, post},
    Router,
};
use std::env;
mod handlers;


#[tokio::main]
#[allow(unused)]
async fn main() {
    let app = Router::new()
        .route("/", get(handlers::root::root))
    ;
    let port = env::var("PORT").unwrap();
    let addr = format!("0.0.0.0:{}", port);
    let listner = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listner, app);
}