use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response}, extract,
};
use tracing::info;

use super::abi::GenerateProofReuqest;

pub async fn generate_proof_handler(
   extract::Json(request): extract::Json<GenerateProofReuqest>,
) -> impl IntoResponse {
    info!("generate_proof_handler: {:?}", request);
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("proofs generated"))
        .unwrap()
}
