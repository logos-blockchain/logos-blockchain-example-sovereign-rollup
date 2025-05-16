# Nomos Sovereign Rollup Example

This repository contains a proof-of-concept implementation of a Risc0-based sovereign rollup showcased by Nomos during IFT All-Hands 2025 in Split. The system demonstrates a complete end-to-end sovereign rollup architecture with EVM compatibility.

## Overview

This implementation consists of three main components:

1. **Sequencer Node**: An Reth-based execution node that processes transactions and submits blocks to Nomos.
2. **Prover**: Generates zero-knowledge proofs for block execution, ensuring the correctness of state transitions.
3. **Light Node**: Verifies proofs and block data, enabling trustless verification of the rollup state.

The system integrates with a fork of zeth (an Ethereum STF):
👉 [logos-co/zeth](https://github.com/logos-co/zeth)

Additionally, the repository includes a fully functioning Uniswap example:

- [Uniswap's Contracts](./nomos-example-uniswap-contracts/) - Smart contracts for the DEX
- [Nomoswap: Uniswap Frontend](./nomos-example-uniswap/) - Web interface for interacting with the DEX
- [Blockscout: Block Explorer](./nomos-example-blockscout/) - Block explorer for the rollup

## Architecture

```
+----------------+      +--------------+     +----------------+
|   Sequencer    |----->|    Prover    |<----|   Light Node   |
|    (EVM)       |      |   (Risc0)    |     |  (Verifier)    |
+----------------+      +--------------+     +----------------+
        |                                            |
        v                                            v
+----------------+                           +----------------+
|  Nomos Node    |<--------------------------|  Consensus/DA  |
|  (DA Layer)    |                           |  Verification  |
+----------------+                           +----------------+
```

The system works as follows:
1. Sequencer processes transactions and sends blocks to the Nomos data availability layer
2. Prover generates ZK proofs of block execution
3. Light Node fetches blocks from the sequencer and verifies proofs
4. Light Node confirms block data is available on Nomos


## Prerequisites

- Rust
- Cargo
- Just
- Risc0
- Access to a Nomos node

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/your-org/nomos-example-sovereign.git --recurse-submodules
   cd nomos-example-sovereign
   ```

2. Build zeth:
   ```bash
   cd zeth
   just build
   ```
   GPU acceleration (Apple Metal and CUDA) is supported and greatly improves performance. To enable it, compile zeth appropriately with:
   ```bash
    just metal
    ```
    or 
    ```bash
    just cuda
    ```


2. Build the remaining components:
   ```bash
   cargo build --release
   ```

## Running the Sovereign Rollup

### 1. Start the Sequencer Node

The sequencer node processes transactions and submits blocks to Nomos:

```bash
NOMOS_EXECUTOR="https://nomos.url" \
NOMOS_USER=your_username \
NOMOS_PASSWORD=your_password \
cargo run --release --bin evm-sequencer-node
```

Environment variables:
- `NOMOS_EXECUTOR`: URL to the Nomos testnet node
- `NOMOS_USER` and `NOMOS_PASSWORD`: Authentication credentials for the Nomos node, if required

The sequencer exposes RPC endpoints on:
- HTTP: http://localhost:8545
- WebSocket: wss://localhost:8546

### 2. Start the Prover

The prover generates zero-knowledge proofs for block execution:

```bash
cargo run --release --bin evm-prover -- \
  --rpc http://localhost:8545 \
  --start-block 0 \
  --batch-size 10 \
  --interval 5 \
  --zeth-binary-dir /path/to/zeth/bin
```

Parameters:
- `--rpc`: URL to the sequencer's RPC endpoint
- `--start-block`: Block number to start proving from
- `--batch-size`: Number of blocks to prove in a batch
- `--interval`: Polling interval in seconds
- `--zeth-binary-dir`: Path to the directory containing the zeth binary

The prover service will be available at: http://localhost:8070

### 3. Start the Light Node [experimental]

The light node validates proofs and verifies block data. Please note that this is an experimental feature and may not be fully functional.

```bash
cargo run --release --bin evm-lightnode -- \
  --rpc http://localhost:8545 \
  --ws-rpc wss://localhost:8546 \
  --prover-url http://localhost:8070 \
  --nomos-node https://nomos.url \
  --batch-size 10 \
  --zeth-binary-dir /path/to/zeth
```

Parameters:
- `--rpc`: HTTP RPC endpoint of the sequencer
- `--ws-rpc`: WebSocket RPC endpoint of the sequencer
- `--prover-url`: URL to the prover service
- `--nomos-node`: URL to the Nomos testnet node
- `--batch-size`: Number of blocks in a proof batch
- `--zeth-binary-dir`: Path to the directory containing the zeth binary

## Example Application: Deploying Uniswap

Follow these steps to deploy and interact with Uniswap on the sovereign rollup:

1. Deploy the [Uniswap's Contracts](./nomos-example-uniswap-contracts/)
   ```bash
   cd nomos-example-uniswap-contracts
   # Follow instructions in the README
   ```

2. Deploy [Nomoswap](./nomos-example-uniswap/)
   ```bash
   cd nomos-example-uniswap
   # Follow instructions in the README
   ```

3. (Optional) Deploy [Blockscout](./nomos-example-blockscout/)
   ```bash
   cd nomos-example-blockscout
   # Follow instructions in the README
   ```

Each component's directory contains a detailed README with specific setup instructions.

