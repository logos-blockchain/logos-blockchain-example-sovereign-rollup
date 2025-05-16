use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs;

#[derive(Deserialize)]
pub struct ProofRequest {
    block_start: u64,
    block_count: u64,
}

/// Handler for GET /
pub async fn serve_proof(Query(query): Query<ProofRequest>) -> Response {
    let file_name = format!(
        "{}-{}.zkp",
        query.block_start,
        query.block_count + query.block_start
    );

    let path = PathBuf::from(&file_name);

    // Read file contents
    match fs::read(&path).await {
        Ok(bytes) => (StatusCode::OK, bytes).into_response(),
        Err(err) => {
            let status = if err.kind() == std::io::ErrorKind::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, format!("Error reading file: {}", err)).into_response()
        }
    }
}
