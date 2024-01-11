use std::sync::{Arc, Mutex};

use axum::{extract, http::StatusCode, Json};
use bellperson::groth16;
use ironfish_rust::sapling_bls12::SAPLING;
use ironfish_zkp::proofs::{MintAsset, Output, Spend};
use rand::thread_rng;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tracing::{error, info};

use super::abi::{GenerateProofRep, GenerateProofReq};

pub async fn generate_proof_handler(
    extract::Json(request): extract::Json<GenerateProofReq>,
) -> Result<Json<GenerateProofRep>, StatusCode> {
    info!("calling generate_proof_handler");
    let all_succeed = Arc::new(Mutex::new(true));
    let spend_proofs = request
        .spend_circuits
        .par_iter()
        .map(|bytes| {
            let proof_bytes = if let Ok(spend) = Spend::read(&bytes[..]) {
                let proof =
                    groth16::create_random_proof(spend, &SAPLING.spend_params, &mut thread_rng());
                if let Ok(proof) = proof {
                    let mut writer = vec![];
                    proof.write(&mut writer).unwrap();
                    Some(writer)
                } else {
                    None
                }
            } else {
                None
            };
            if proof_bytes.is_none() {
                *all_succeed.lock().unwrap() = false;
                error!("generate spend proof failed");
                return vec![];
            }
            proof_bytes.unwrap()
        })
        .collect();
    let output_proofs = if *all_succeed.lock().unwrap() {
        request
            .output_circuits
            .par_iter()
            .map(|bytes| {
                let proof_bytes = if let Ok(output) = Output::read(&bytes[..]) {
                    let proof = groth16::create_random_proof(
                        output,
                        &SAPLING.output_params,
                        &mut thread_rng(),
                    );
                    if let Ok(proof) = proof {
                        let mut writer = vec![];
                        proof.write(&mut writer).unwrap();
                        Some(writer)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if proof_bytes.is_none() {
                    *all_succeed.lock().unwrap() = false;
                    error!("generate output proof failed");
                    return vec![];
                }
                proof_bytes.unwrap()
            })
            .collect()
    } else {
        vec![]
    };
    let mint_asset_proofs = if *all_succeed.lock().unwrap() {
        request
            .mint_asset_circuits
            .par_iter()
            .map(|bytes| {
                let proof_bytes = if let Ok(mint_asset) = MintAsset::read(&bytes[..]) {
                    let proof = groth16::create_random_proof(
                        mint_asset,
                        &SAPLING.mint_params,
                        &mut thread_rng(),
                    );
                    if let Ok(proof) = proof {
                        let mut writer = vec![];
                        proof.write(&mut writer).unwrap();
                        Some(writer)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if proof_bytes.is_none() {
                    *all_succeed.lock().unwrap() = false;
                    error!("generate mint asset proof failed");
                    return vec![];
                }
                proof_bytes.unwrap()
            })
            .collect()
    } else {
        vec![]
    };
    if !*all_succeed.lock().unwrap() {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(Json(GenerateProofRep {
        spend_proofs,
        output_proofs,
        mint_asset_proofs,
    }))
}
