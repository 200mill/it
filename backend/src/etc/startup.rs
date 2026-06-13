use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::etc::llm::{self, LlmConfig};

/// Shared, cheaply-cloneable application state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub http: reqwest::Client,
    pub llm: LlmConfig,
}

/// Connect to Postgres, run pending migrations, and assemble the shared state.
pub async fn init() -> AppState {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = connect_with_retry(&url).await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations failed");

    AppState {
        pool,
        http: reqwest::Client::new(),
        llm: llm::from_env(),
    }
}

/// Connect to Postgres, retrying for a while so the service can start before the database is ready
/// (e.g. under docker-compose). Panics only after exhausting all attempts.
async fn connect_with_retry(url: &str) -> PgPool {
    const ATTEMPTS: u32 = 30;
    const DELAY: Duration = Duration::from_secs(2);

    for attempt in 1..=ATTEMPTS {
        match PgPoolOptions::new().max_connections(5).connect(url).await {
            Ok(pool) => return pool,
            Err(e) if attempt < ATTEMPTS => {
                eprintln!("postgres not ready (attempt {attempt}/{ATTEMPTS}): {e}; retrying...");
                tokio::time::sleep(DELAY).await;
            }
            Err(e) => panic!("failed to connect to Postgres after {ATTEMPTS} attempts: {e}"),
        }
    }
    unreachable!()
}
