use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use clap::Parser;
use evm_lightnode::{NomosClient, nomos::HeaderId, proofcheck, Credentials};
use futures::Stream;
use futures::StreamExt;
use nomos_core::da::{BlobId, DaEncoder};
use serde::{Deserialize, Serialize};
use std::error;
use std::num::NonZero;
use std::path::{Path, PathBuf};
use tracing_subscriber::{EnvFilter, fmt};
use url::Url;
use anyhow::Result;
use tokio::time::{sleep,  Duration};

#[derive(Parser, Debug)]
#[clap(author, version, about = "Light Node validator")]
struct Args {
    #[clap(long, default_value = "info")]
    log_level: String,

    #[clap(long, default_value = "http://localhost:8545")]
    rpc: Url,

    #[clap(long, default_value = "wss://localhost:8546")]
    ws_rpc: Url,

    #[clap(long, default_value = "http://localhost:8070")]
    prover_url: Url,

    #[clap(long, default_value = TESTNET_EXECUTOR)]
    nomos_node: Url,

    #[clap(long, default_value = "10")]
    batch_size: u64,

    #[clap(long)]
    zeth_binary_dir: Option<PathBuf>,
}

const TESTNET_EXECUTOR: &str = "https://testnet.nomos.tech/node/3/";

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let args = Args::parse();

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&args.log_level));

    fmt::fmt().with_env_filter(filter).with_target(false).init();

    let zone_blocks = follow_sz(args.ws_rpc.clone()).await?;
    let (tx, da_blobs) = tokio::sync::mpsc::channel::<BlobId>(MAX_BLOBS);
    tokio::spawn(check_blobs(
        NomosClient::new(
            args.nomos_node.clone(),
            Credentials {
                username: "user".to_string(),
                password: Some("password".to_string()),
            },
        ),
        tx
    ));

    verify_zone_stf(
        args.batch_size,
        args.rpc.clone(),
        args.prover_url.clone(),
        args.zeth_binary_dir.as_deref().unwrap_or_else(|| Path::new("zeth")),
        zone_blocks,
        tokio_stream::wrappers::ReceiverStream::new(da_blobs),
    )
    .await?;


    Ok(())
}

const MAX_BLOBS: usize = 1 << 10;
const MAX_PROOF_RETRIES: usize = 5;

async fn verify_zone_stf(
    batch_size: u64,
    rpc: Url,
    prover_url: Url,
    zeth_binary_dir: &Path,
    blocks: impl Stream<Item = (u64, BlobId)>,
    included_blobs: impl Stream<Item = BlobId>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut blobs_on_consensus = lru::LruCache::new(NonZero::new(MAX_BLOBS).unwrap());
    let mut sz_blobs = lru::LruCache::new(NonZero::new(MAX_BLOBS).unwrap());

    tokio::pin!(blocks);
    tokio::pin!(included_blobs);
    loop {
        let to_verify = tokio::select! {
            Some((block_n, blob_id)) = blocks.next() => {
                if blobs_on_consensus.pop(&blob_id).is_some() {
                    tracing::debug!("Block confirmed on consensus: {:?}", block_n);
                    Some((block_n, blob_id))
                } else  {
                    if let Some(expired) = sz_blobs.push(blob_id, block_n) {
                        tracing::warn!("Block was not confirmed on mainnet: {:?}", expired);
                    }
                    None
                }
            }
            Some(blob_id) = included_blobs.next() => {
                if let Some(block_n) = sz_blobs.pop(&blob_id) {
                    tracing::debug!("Block confirmed on consensus: {:?}", block_n);
                    Some((block_n, blob_id))
                } else {
                    // Blobs coming from other zones are not going to be confirmed and
                    // should be removed from the cache.
                    // It is highly unlikely that a zone blob is seen first on consensus and
                    // later enough on the sz rpc to be dropped from the cache, although
                    // it is possible.
                    // Do something here if you want to keep track of those blobs.
                    let _ = blobs_on_consensus.push(blob_id, ());
                    None
                }
            }
        };

        if let Some((block_n, blob_id)) = to_verify {
            let rpc = rpc.clone();
            let prover_url = prover_url.clone();
            let zeth_binary_dir = zeth_binary_dir.to_path_buf();
            tokio::spawn(async move {
                verify_blob(
                    block_n,
                    blob_id,
                    batch_size,
                    &rpc,
                    &prover_url,
                    &zeth_binary_dir,
                )
                .await
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to verify blob: {:?}", e);
                });
            });
        }
    }
}

async fn verify_blob(
    block_number: u64,
    blob: BlobId,
    batch_size: u64,
    rpc: &Url,
    prover_url: &Url,
    zeth_binary_dir: &Path,
) -> Result<()> {
    // block are proved in batches, aligned to `batch_size`
    let block_number = block_number - block_number % batch_size;
    futures::try_join!(evm_lightnode::da::sampling(blob), async {
        let mut sleep_time = 1;
        for _ in 0..MAX_PROOF_RETRIES {
            if proofcheck::verify_proof(
                block_number,
                batch_size,
                rpc,
                prover_url,
                zeth_binary_dir,
            )
            .await.is_ok() {
                return Ok(());
            }
            
            sleep(Duration::from_secs(sleep_time)).await;
            sleep_time *= 2;
        }
        Err(anyhow::anyhow!("Failed to verify proof after {} retries", MAX_PROOF_RETRIES))
    })?;
    // TODO: reuse rpc results
    Ok(())
}

/// Follow the sovereign zone and return the blob ID of each new produced block
async fn follow_sz(
    ws_rpc: Url,
) -> Result<impl Stream<Item = (u64, BlobId)>, Box<dyn error::Error>> {
    let provider = ProviderBuilder::new().on_ws(WsConnect::new(ws_rpc)).await?;
    // Pub-sub: get a live stream of blocks
    let blocks = provider
        .subscribe_full_blocks()
        .full()
        .into_stream()
        .await?;

    Ok(blocks.filter_map(|block| async move {
        if let Ok(block) = block {
            // poor man's proof of equivalence
            use alloy::consensus::{Block, Signed};
            let block: Block<Signed<_>> = block.into_consensus().convert_transactions();
            let (data, _) = evm_sequencer_node::encode_block(&block.clone().convert_transactions());
            let blob_id = {
                let encoder = kzgrs_backend::encoder::DaEncoder::new(
                    kzgrs_backend::encoder::DaEncoderParams::new(
                        2,
                        false,
                        kzgrs_backend::global::GLOBAL_PARAMETERS.clone(),
                    ),
                );
                // this is a REALLY heavy task, so we should try not to block the thread here
                let heavy_task = tokio::task::spawn_blocking(move || encoder.encode(&data));
                let encoded_data = heavy_task.await.unwrap().unwrap();
                kzgrs_backend::common::build_blob_id(
                    &encoded_data.aggregated_column_commitment,
                    &encoded_data.row_commitments,
                )
            };

            Some((block.header.number, blob_id))
        } else {
            tracing::error!("Failed to get block");
            None
        }
    }))
}

#[derive(Debug, Serialize, Deserialize)]
struct CryptarchiaInfo {
    tip: HeaderId,
}

/// Return blobs confirmed on Nomos.
async fn check_blobs(
    nomos_client: NomosClient,
    sink: tokio::sync::mpsc::Sender<BlobId>,
) -> Result<()> {
    // TODO: a good implementation should follow the different forks and react to possible
    // reorgs.
    // We don't currently habe a food api to follow the chain externally, so for now we're just
    // going to simply follow the tip.
    let mut current_tip = HeaderId::default();
    loop {
        let info = nomos_client.get_cryptarchia_info().await?;
        if info.tip != current_tip {
            current_tip = info.tip;
            tracing::debug!("new tip: {:?}", info.tip);
            let blobs = nomos_client.get_block(info.tip).await?.blobs;

            if blobs.is_empty() {
                tracing::debug!("No blobs found in block");
                continue;
            }

            for blob in blobs {
                sink.send(blob).await?;
            }
        } else {
            tracing::trace!("No new tip, sleeping...");
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
}
