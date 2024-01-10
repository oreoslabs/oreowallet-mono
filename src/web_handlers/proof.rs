use axum::{extract, http::StatusCode, Json};
use bellperson::groth16;
use ironfish_rust::sapling_bls12::SAPLING;
use ironfish_zkp::proofs::Spend;
use rand::thread_rng;
use tracing::info;

use super::abi::{GenerateProofRep, GenerateProofReq};

pub async fn generate_proof_handler(
    extract::Json(request): extract::Json<GenerateProofReq>,
) -> Result<Json<GenerateProofRep>, StatusCode> {
    info!("calling generate_proof_handler");
    let mut spend_proofs = vec![];
    for reader in request.spends {
        let spend = Spend::read(&reader[..]).unwrap();
        let proof = groth16::create_random_proof(spend, &SAPLING.spend_params, &mut thread_rng());
        if let Ok(proof) = proof {
            let mut writer = vec![];
            proof.write(&mut writer).unwrap();
            spend_proofs.push(writer);
        } else {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    Ok(Json(GenerateProofRep { spend_proofs }))
}
