use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs;

// Tutorial code
