use axum::{Router, routing::get};
use clap::Parser;
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::{net::SocketAddr, path::PathBuf, process::Command, thread, time::Duration};
use tracing::{debug, error, info};
use tracing_subscriber::{EnvFilter, fmt};

mod http;

// Tutorial code
