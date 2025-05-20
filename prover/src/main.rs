use axum::{Router, routing::get};
use clap::Parser;
use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::{net::SocketAddr, path::PathBuf, process::Command, thread, time::Duration};
use tracing::{debug, error, info};
use tracing_subscriber::{EnvFilter, fmt};

mod http;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Ethereum Proof Generation Tool")]
struct Args {
    #[clap(long, default_value = "http://localhost:8545")]
    rpc: String,

    #[clap(long, default_value = "5")]
    start_block: u64,

    #[clap(long, default_value = "10")]
    batch_size: u64,

    #[clap(long, default_value = "5")]
    interval: u64,

    #[clap(long)]
    zeth_binary_dir: Option<String>,

    #[clap(long, default_value = "info")]
    log_level: String,
}

fn run_ethereum_prove(
    rpc: &str,
    block_number: u64,
    batch_size: u64,
    zeth_binary_dir: Option<String>,
    log_level: &str,
) -> Result<(), String> {
    info!(
        "Running Ethereum prove for blocks {}-{}",
        block_number,
        block_number + batch_size - 1
    );

    let mut binary_path = if let Some(dir) = &zeth_binary_dir {
        PathBuf::from(dir)
    } else {
        debug!("No binary directory provided, trying current directory");
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?
    };

    binary_path.push("zeth-ethereum");

    if !binary_path.exists() {
        return Err(format!("Binary not found at: {}", binary_path.display()));
    }

    let output = Command::new(&binary_path)
        .env("RUST_LOG", log_level)
        .args([
            "prove",
            &format!("--rpc={}", rpc),
            &format!("--block-number={}", block_number),
            &format!("--block-count={}", batch_size),
        ])
        .output()
        .map_err(|e| format!("Failed to execute zeth-ethereum prove: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("ethereum prove command failed: {}", stderr);
        return Err(format!(
            "ethereum prove command failed with status: {}\nStderr: {}",
            output.status, stderr
        ));
    }

    info!("Successfully processed batch");
    Ok(())
}

fn get_latest_block(client: &Client, rpc: &str) -> Result<u64, String> {
    debug!("Checking latest block height...");

    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    });

    let response = client
        .post(rpc)
        .json(&request_body)
        .send()
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Request failed with status: {}", response.status()));
    }

    let response_json: Value = response
        .json()
        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

    let block_hex = response_json
        .get("result")
        .and_then(Value::as_str)
        .ok_or("Failed to parse response")?
        .trim_start_matches("0x");

    let block_number = u64::from_str_radix(block_hex, 16)
        .map_err(|e| format!("Failed to parse hex block number: {}", e))?;

    debug!("Latest block: {}", block_number);
    Ok(block_number)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    std::thread::spawn(move || {
        if let Err(e) = run_server() {
            error!("Error running server: {}", e);
        }
    });

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&args.log_level));

    fmt::fmt().with_env_filter(filter).with_target(false).init();

    info!("Starting Ethereum prover...");

    let client = Client::new();
    let mut current_block = args.start_block;

    loop {
        match get_latest_block(&client, &args.rpc) {
            Ok(latest_block) => {
                if latest_block >= current_block + args.batch_size {
                    info!(
                        "New blocks available. Current: {}, Latest: {}",
                        current_block, latest_block
                    );

                    match run_ethereum_prove(
                        &args.rpc,
                        current_block,
                        args.batch_size,
                        args.zeth_binary_dir.clone(),
                        &args.log_level,
                    ) {
                        Ok(_) => {
                            current_block += args.batch_size;
                            info!("Updated current block to {}", current_block);
                        }
                        Err(e) => {
                            error!("Error running prover: {}", e);
                        }
                    }
                } else {
                    info!(
                        "No new blocks to process. Current: {}, Latest: {}, sleeping...",
                        current_block, latest_block
                    );
                    thread::sleep(Duration::from_secs(args.interval));
                }
            }
            Err(e) => {
                error!("Error getting latest block: {}", e);
                thread::sleep(Duration::from_secs(args.interval));
            }
        }
    }
}

#[tokio::main]
async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    // Build our application with a route
    let app = Router::new().route("/", get(http::serve_proof));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8070));
    // Run it on localhost:8070
    tracing::info!("Serving files on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
