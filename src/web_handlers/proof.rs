use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateProofReuqest {
    pub circuits: Vec<u32>,
}

pub async fn generate_proof_handler(
    Json(request): Json<GenerateProofReuqest>,
) -> impl IntoResponse {
    info!("generate_proof_handler: {:?}", request);
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("proofs generated"))
        .unwrap()
}
