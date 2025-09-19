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

// Tutorial code
