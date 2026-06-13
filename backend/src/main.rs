use axum::{
    Router,
    routing::{get, post, put},
};

mod error;
mod etc;
mod handlers;
mod models;

#[tokio::main]
async fn main() {
    let state = etc::startup::init().await;

    let app = Router::new()
        .route("/", get(handlers::root::root))
        .route(
            "/zakonim",
            get(handlers::zakonim::list).post(handlers::zakonim::register),
        )
        .route(
            "/issues",
            get(handlers::issue::list).post(handlers::issue::create),
        )
        .route(
            "/issues/{id}",
            get(handlers::issue::get).patch(handlers::issue::edit),
        )
        .route(
            "/issues/{id}/comments",
            get(handlers::comment::list).post(handlers::comment::create),
        )
        .route(
            "/issues/{id}/discord-messages/{key}",
            put(handlers::issue::set_discord_message),
        )
        .route(
            "/discord/messages/{message_id}",
            put(handlers::discord::upsert).delete(handlers::discord::delete),
        )
        .route("/summary/drafts", post(handlers::summary::create))
        .route(
            "/summary/drafts/{id}/messages",
            post(handlers::summary::message),
        )
        .route(
            "/summary/drafts/{id}/confirm",
            post(handlers::summary::confirm),
        )
        .with_state(state);

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:80".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
