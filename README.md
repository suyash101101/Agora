## Agora Parachain Template – Verifiable Off‑Chain Computation via Commit‑Reveal and XCM

Agora is a Polkadot parachain template extended with a verifiable computation marketplace. It enables parachains to outsource off‑chain jobs (API fetches, computations) to a network of staked workers. Results are verified on‑chain using a crypto‑economic commit‑reveal game. Cross‑parachain requests and result delivery use XCM.

### Why Agora
- Trusted hardware (e.g., SGX) has vulnerabilities; pure OCWs are untrusted. Agora uses crypto‑economic incentives and majority agreement to create a Schelling point for correctness.
- Works across parachains: submit from chain A, compute on chain B, return verified result via XCM.

### App Workflow

![App Workflow](assets/App_Workflow.png)

## High‑Level Architecture

- Runtime (`runtime/`): FRAME‑based runtime with balances, collator pallets, XCM helpers, and the `agora` pallet. XCM is configured for local testing with permissive filters.
- Node (`node/`): Collator binary that runs the runtime, exposes RPC, and connects to the relay chain.
- Agora Pallet (`pallets/agora/`):
  - Job lifecycle: submit -> commit -> reveal -> finalize
  - Staking and bounty holds; rewards/slashing; simple reputation
  - Off‑Chain Worker (OCW) helpers for job execution and commit/reveal guidance
  - XCM client/handler for cross‑parachain job requests and result notifications
- Zombienet configs (`zombienet-*.toml`): Local networks with relay chain + 2 parachain instances for XCM demos.
- Scripts (`JS/`): Utilities to fund sovereign accounts, open HRMP channels, and run a cross‑chain job demo.

## Repository Layout

- `node/` – Collator service (AURA consensus, networking, RPC)
- `runtime/` – Runtime types, pallet composition, XCM config (`runtime/src/configs/xcm_config.rs`)
- `pallets/agora/` – Verifiable computation pallet
  - `lib.rs` – Storage, events, errors, extrinsics, hooks
  - `functions.rs` – Consensus, rewards/slashing, lifecycle processing
  - `ocw.rs` – Off‑chain worker execution helpers (simulated HTTP/compute), commit/reveal guidance
  - `xcm_job_client.rs` – Build/send XCM job requests, receive results
  - `xcm_handler.rs` – Handle incoming XCM job submissions, send results back
  - `types.rs` – Job, worker, commit/reveal, OCW types
- `zombienet-xcm.toml` – Relay + two parachains (IDs 1000, 2000) with XCM tracing enabled
- `JS/setup-hrmp.js` – HRMP channel setup, sovereign funding, and `agora.requestRemoteJob` demo

## Build

Prereqs: Rust toolchain (nightly as per `rust-toolchain.toml`), wasm32 target, Node.js (for scripts), and a `polkadot` relay binary.

```bash
# From repo root
cargo build --release -p node

# Optional: install the collator binary somewhere on PATH
sudo cp target/release/parachain-template-node /usr/local/bin/

# Ensure a polkadot relay node is available on PATH (or provide absolute path in Zombienet)
sudo cp polkadot /usr/local/bin/polkadot
```

## Run: Local Multi‑Chain Network (Zombienet)

This repo includes `zombienet-xcm.toml` to boot a relay chain (rococo‑local) and two parachain instances for XCM testing.

```bash
# Example with zombienet (ensure you have it installed)
zombienet spawn zombienet-xcm.toml
```

Key ports (from `zombienet-xcm.toml`):
- Relay RPC/ws: 9944/9988
- Para 1000 RPC/ws: 9946/9990
- Para 2000 RPC/ws: 9947/9991

## HRMP + XCM Demo

Use the provided Node.js script to fund sovereign accounts, open HRMP channels, and dispatch a cross‑chain job request.

```bash
pnpm i || npm i
node JS/setup-hrmp.js
```

The script performs:
- Funds sovereign accounts (relay and cross‑parachain)
- Opens HRMP channels 1000 ↔ 2000 using XCM `Transact`
- Sends `agora.requestRemoteJob` on Para 1000 to Para 2000
- Waits for execution and verifies job creation and events on Para 2000

## Agora Pallet – On‑Chain API

Extrinsics (selected):
- `agora.submitJob(job_type_id, input_data, bounty)` – Submit a local job with a bounty hold
- `agora.registerWorker(stake)` / `agora.unregisterWorker()` – Worker enrollment with staking
- `agora.commitResult(job_id, salt, result_hash)` – Commit phase
- `agora.revealResult(job_id, result)` – Reveal phase; pallet verifies salted hash
- `agora.finalizeJob(job_id)` – Finalizes by consensus (auto‑finalization also runs in `on_initialize`)
- `agora.requestRemoteJob(dest_para_id, input_data, bounty, program_hash)` – Send cross‑para job via XCM
- `agora.receiveRemoteJobResult(job_id_hash, result_hash, success)` – Called via XCM by the executor chain
- `agora.xcmHandleJobSubmission(sender, input_data, bounty, job_id_hash, program_hash)` – XCM entrypoint to accept jobs

Events include job submission, commits/reveals, finalization, rewards/slashing, and XCM‑related notifications.

Storage highlights:
- `Jobs`, `Workers`, `Commits`, `Reveals`, `Results`, `PendingJobs`, `RemoteJobInfo`, `NextJobId`, `JobNonce`.

Consensus and rewards:
- Majority of reveals determine the canonical result.
- Honest workers split the bounty; dishonest workers lose reputation and have stake reduced.



## Off‑Chain Worker (OCW)

- Scans for pending jobs and simulates execution for:
  - API requests: returns simulated payloads (upgradeable to real HTTP with proper signing/validation)
  - Computation jobs: hash/math/crypto/json toy tasks
- Generates salted commits and logs ready‑to‑submit `commitResult`/`revealResult` transactions.
- Automation of signed extrinsic submission can be enabled in future work.

![OCW Workflow](assets/OCW_Workflow.png)

## XCM Flow (Request → Execute → Deliver)

1) Chain A calls `agora.requestRemoteJob(B, input, bounty, program_hash)`
   - Reserves bounty locally; builds XCM with `Transact` to Chain B `xcmHandleJobSubmission`.
2) Chain B creates a local job, holds bounty on sender’s account, emits `XcmJobSubmitted`.
3) Workers execute, commit, reveal; pallet finalizes with consensus.
4) Upon finalization, the pallet computes the result hash and calls `maybe_send_remote_result`,
   which sends an XCM `Transact` back to Chain A `receiveRemoteJobResult(...)`.
5) Chain A releases local hold and emits `RemoteJobCompleted` or `RemoteJobFailed`.

![XCM Workflow](assets/XCM_Workflow.png)

## Configuration and Parameters

Runtime (`runtime/src/lib.rs`) sets:
- Block time: 6s; weight/fee mapping
- `Agora` constants: `MinWorkerStake`, `MinJobBounty`, commit/reveal durations, limits
- XCM router: Parent UMP + XCMP; permissive `Barrier` and `Everything` filters for local testing

Harden for production:
- Restrict XCM execute/send filters; require paid execution with proper fee trading
- Tighten `LocationToAccountId` conversions and origins
- Replace simulated OCW HTTP with a secure, signed, and verifiable approach

## Troubleshooting

- XCM not delivered: ensure HRMP channels 1000 ↔ 2000 are open (relay `hrmpChannels`), and sovereign accounts funded.
- Insufficient balance/errors: adjust `UNIT`/bounty values or fund accounts via balances pallet.
- OCW not revealing: ensure commit deadline passed and reveal is within the reveal window.
- Pallet index/call index in XCM encode must match the runtime (here, `Agora` at index 51 for this template).

## Security Notes

- This template is for development. Many XCM filters are permissive.
- OCW HTTP is simulated and not a secure oracle. For production, implement authenticated requests, verification, and anti‑spam.


