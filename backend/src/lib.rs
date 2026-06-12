use axum::{
    routing::{get, post},
    Router,
};

mod handlers;
mod etc;


#[tokio::main]
#[allow(unused)]
async fn main() {
    

    let app = Router::new()
        .route("/", get(handlers::root::root))
    ;

    let listner = tokio::net::TcpListener::bind("0.0.0.0:80").await.unwrap();
    axum::serve(listner, app);
}