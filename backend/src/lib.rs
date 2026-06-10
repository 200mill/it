#[allow(unused_imports)]
use axum::{
    routing::{get, post},
    Router,
};
use std::env;


#[tokio::main]
#[allow(unused)]
async fn main() {
    let app = Router::new()
    ;
    let port = env::var("PORT").unwrap();
    let addr = format!("0.0.0.0:{}", port);
    let listner = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listner, app);
}