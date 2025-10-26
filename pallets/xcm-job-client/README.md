# XCM Job Client Pallet

A Substrate pallet that enables parachains to send job submission requests to remote Agora parachains via XCM (Cross-Consensus Messaging) and receive results back.

## Overview

This pallet acts as a client-side interface for submitting computational jobs to remote Agora parachains. It handles:

- Job request creation and XCM message construction
- Payment reservation and management
- Job result reception from remote chains
- Job cancellation and refunds

## Features

- **Remote Job Submission**: Send job requests with input data, bounty, and program hash to any Agora parachain
- **Payment Management**: Automatically reserves bounty on job submission and unreserves on completion/cancellation
- **XCM Integration**: Seamlessly integrates with Polkadot's XCM for cross-chain communication
- **Job Tracking**: Maintains a registry of pending jobs with their status
- **Result Reception**: Receives and processes job results from remote chains

## Dispatchable Functions

### `request_remote_job`
Sends a job submission request to a remote Agora parachain.

**Parameters:**
- `origin`: The account requesting the job
- `dest_para_id`: The destination parachain ID running Agora
- `input`: The job input data (bounded by `MaxInputBytes`)
- `bounty`: The payment amount for the job
- `program_hash`: Hash of the program to execute

**Effects:**
- Reserves the bounty from the sender's balance
- Generates a unique job ID
- Stores the job as pending
- Sends XCM message to the destination parachain
- Emits `RemoteJobRequested` event

### `receive_remote_job_result`
Receives job results from a remote Agora parachain (called via XCM).

**Parameters:**
- `origin`: Should be the sovereign account of the remote parachain
- `job_id`: The unique identifier of the completed job
- `result_hash`: Hash of the job result
- `success`: Whether the job completed successfully

**Effects:**
- Unreserves the bounty
- Removes the job from pending storage
- Emits `RemoteJobCompleted` or `RemoteJobFailed` event

### `cancel_remote_job`
Cancels a pending job request and refunds the bounty.

**Parameters:**
- `origin`: Must be the original job requester
- `job_id`: The unique identifier of the job to cancel

**Effects:**
- Unreserves the bounty
- Removes the job from pending storage

## Configuration

Add to your runtime's `Cargo.toml`:

```toml
pallet-xcm-job-client = { path = "../pallets/xcm-job-client", default-features = false }
```

Configure in your runtime:

```rust
impl pallet_xcm_job_client::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type XcmSender = XcmpQueue;
    type MaxInputBytes = ConstU32<1024>;
    type WeightInfo = pallet_xcm_job_client::weights::SubstrateWeight<Runtime>;
    type XcmExecuteOrigin = EnsureXcm;
}
```

Add to your `construct_runtime!` macro:

```rust
XcmJobClient: pallet_xcm_job_client,
```

## Storage

- `PendingJobs`: Maps job IDs to (requester, dest_para_id, bounty) tuples
- `JobNonce`: Counter for generating unique job IDs

## Events

- `RemoteJobRequested`: Emitted when a job request is sent
- `RemoteJobCompleted`: Emitted when a job completes successfully
- `RemoteJobFailed`: Emitted when a job fails
- `XcmMessageSent`: Emitted when an XCM message is sent successfully

## Errors

- `XcmSendFailed`: Failed to send XCM message
- `InsufficientBalance`: Sender doesn't have enough balance for the bounty
- `JobNotFound`: Referenced job doesn't exist
- `InvalidInput`: Invalid input data provided
- `InputTooLarge`: Input data exceeds maximum size
- `PendingJobExists`: Account already has a pending job
- `Overflow`: Arithmetic overflow occurred

## Usage Example

```rust
use pallet_xcm_job_client;
use frame_support::BoundedVec;

// Submit a job to parachain 2000
let input = BoundedVec::try_from(vec![1, 2, 3, 4]).unwrap();
let bounty = 1000;
let program_hash = sp_core::H256::from_slice(&[0u8; 32]);

XcmJobClient::request_remote_job(
    RuntimeOrigin::signed(sender),
    2000, // destination parachain ID
    input,
    bounty,
    program_hash
)?;
```

## Integration with Agora Pallet

This pallet is designed to work with the Agora pallet on remote parachains. The flow is:

1. Client parachain calls `request_remote_job`
2. XCM message is sent to Agora parachain with payment
3. Agora parachain receives message and calls `submit_job`
4. Workers process the job (commit/reveal)
5. Upon completion, Agora sends result back via XCM
6. Client parachain receives result via `receive_remote_job_result`

## Testing

Run the tests with:

```bash
cargo test -p pallet-xcm-job-client
```

## Benchmarking

Generate weights with:

```bash
cargo build --release --features runtime-benchmarks
./target/release/node benchmark pallet \
    --pallet pallet_xcm_job_client \
    --extrinsic "*" \
    --steps 50 \
    --repeat 20 \
    --output pallets/xcm-job-client/src/weights.rs
```

## License

Unlicense