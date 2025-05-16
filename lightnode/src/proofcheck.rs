use anyhow::Result;
use reqwest::Url;
use std::io::Write;
use std::path::Path;
use tokio::process::Command;
use tracing::{info};

pub async fn verify_proof(
    block_number: u64,
    block_count: u64,
    rpc: &Url,
    prover_url: &Url,
    zeth_bin: &Path,
) -> Result<()> {
    info!(
        "Verifying proof for blocks {}-{}",
        block_number,
        block_number + block_count - 1
    );

    let url = prover_url.join(&format!(
        "/?block_start={}&block_count={}",
        block_number, block_count
    ))?;
    let proof = reqwest::get(url).await?.bytes().await?;

    let mut tempfile = tempfile::NamedTempFile::new()?;
    tempfile.write_all(&proof)?;

    let output = Command::new(zeth_bin)
        .args([
            "verify",
            &format!("--rpc={}", rpc),
            &format!("--block-number={}", block_number),
            &format!("--block-count={}", block_count),
            &format!("--file={}", tempfile.path().display()),
        ])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("zeth-ethereum verify command failed: {}", stderr);
    }

    info!("Proof verification successful");

    Ok(())
}
