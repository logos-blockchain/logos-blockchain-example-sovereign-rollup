use futures::TryStreamExt as _;
use reth::{
    api::{FullNodeTypes, NodePrimitives, NodeTypes},
    cli::Cli,
};
use reth_ethereum::{
    exex::{ExExContext, ExExEvent, ExExNotification},
    node::{EthereumNode, api::FullNodeComponents},
};
use reth_tracing::tracing::info;
use executor_http_client::BasicAuthCredentials;
use evm_sequencer_node::{Processor, NomosDa};

// Tutorial code
