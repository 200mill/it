use axum::{
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use std::env;


pub async fn zakonim(req: ZakonimRequest) -> StatusCode {
    

    return StatusCode::OK;
}

#[derive(Deserialize, Serialize)]
pub struct ZakonimRequest {
    pub id: String,
    pub reason: String,
}