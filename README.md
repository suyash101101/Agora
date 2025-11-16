# Agora 
Verifiable Off‑Chain Computation via Commit‑Reveal and XCM

 

<p align="center">
  <img src="assets/Screenshot%202025-11-04%20at%207.31.33%E2%80%AFPM.png" alt="Agora Logo" width="200" />
</p>
 
 ## System Architecture

![System Architecture](assets/WhatsApp%20Image%202025-11-04%20at%2020.18.35.jpg)

Agora is a Polkadot parachain template extended with a verifiable computation marketplace. It enables parachains to outsource off‑chain jobs (API fetches, computations) to a network of staked workers. Results are verified on‑chain using a crypto‑economic commit‑reveal game. Cross‑parachain requests and result delivery use XCM. This instance has been obtained with paraid 5024 on the Paseo test network.

### Live Walkthough
[Watch Agora Walkthrough](https://app.supademo.com/demo/cm6xuj2sr01o8pegv3s411vap)

### Video Demo
[Watch Agora Demo Video](https://www.youtube.com/watch?v=3V9ne5yBJFs&t=2s)

### Why Agora
- *Cryptoeconomic Verification Without TEEs:* Two-phase commit-reveal protocol (⁠ H(salt || result) ⁠) with on-chain hash verification prevents post-commitment manipulation. Byzantine fault tolerance via majority consensus without SGX dependencies.
  
- *Incentive-Aligned Worker Marketplace:* Staked workers face dual penalties—economic slashing + persistent reputation decay. Honest majority splits bounties; dishonest actors suffer compounding penalties, making long-term defection irrational.

- *XCM-Native Interoperability:* Cross-parachain job submission via ⁠ Transact ⁠ instruction, sovereign account fee payment, asynchronous result callbacks through HRMP channels. Native multi-asset bounty support via ⁠ MultiLocation ⁠ addressing without bridge trust assumptions.

- *Hybrid Execution Model:* OCWs handle I/O-bound tasks (API fetches, crypto ops) off-chain; only commit hashes and reveals touch on-chain storage. Deterministic ⁠ on_initialize ⁠ settlement ensures reproducible consensus, reward distribution, and reputation updates across all nodes—critical for cross-chain result verification.
  
## Use Cases:
⁠•⁠ *Oracle data aggregation*: Request external API data (price feeds, weather, sports scores) from multiple workers, reach consensus on the correct value, and deliver verified results to DeFi protocols.

•⁠ *Decentralized computation*: Offload intensive computations (cryptographic operations, data processing, ML inference) to a distributed worker network without relying on a single centralized service.

•⁠ *Cross-chain data verification*: Enable parachains to request computation from other chains in the Polkadot ecosystem, leveraging specialized compute resources while maintaining trust through majority consensus.

•⁠ *API reliability*: When a single API endpoint is unreliable, multiple workers can fetch from different sources or endpoints, with the majority result being the canonical answer.

## Why Polkadot Architecture is Ideal:

•⁠  ⁠*Native cross-chain communication*: XCM (Cross-Consensus Message Format) enables seamless job submission and result delivery between parachains without bridges or wrapping. Agora leverages this to create a truly interoperable computation marketplace.

•⁠  ⁠*Shared security model*: Parachains inherit security from the relay chain validators, eliminating the need for Agora to bootstrap its own validator set. This reduces costs and increases trust for users submitting jobs.

•⁠  ⁠*Sovereign execution environment*: As a parachain, Agora can customize its runtime specifically for computation marketplace logic (commit-reveal timing, reward distribution, slashing parameters) without constraints from other chains.

•⁠  ⁠*Economic interoperability*: Polkadot's native asset transfer via XCM allows job creators to pay bounties in their own parachain's token, while workers receive rewards in Agora's native token—all atomic and trustless.

•⁠  ⁠*Scalability through specialization*: Dedicated parachain slot allows Agora to optimize for high-throughput job processing without competing for block space with other applications. OCWs can process jobs off-chain and only submit commits/reveals on-chain.

•⁠  ⁠*Future-proof upgrades*: Runtime upgrades enable Agora to evolve its consensus mechanism, add new job types, or adjust economic parameters without hard forks, maintaining backward compatibility.

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

### App Workflow

![App Workflow](assets/App_Workflow.png)

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
 cargo build --release -p parachain-template-node --bin parachain-template-node

# Optional: install the collator binary somewhere on PATH
sudo cp target/release/parachain-template-node /usr/local/bin/

# Ensure a polkadot relay node is available on PATH (or provide absolute path in Zombienet)
sudo cp polkadot /usr/local/bin/polkadot
```

## Run: Local Multi‑Chain Network (Zombienet)

This repo includes `zombienet-xcm.toml` to boot a relay chain (rococo‑local) and two parachain instances for XCM testing.

```bash
# Example with zombienet (ensure you have it installed)
zombienet -p native spawn zombienet-xcm.toml
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
- Debug.js to verify the status of the parachains.

## Agora Pallet – On‑Chain API

Extrinsics (selected):
- `agora.submitJob(job_type_id, input_data, bounty, reputation)` – Submit a local job with a bounty hold
- `agora.registerWorker(stake)` / `agora.unregisterWorker()` – Worker enrollment with staking
- `agora.commitResult(job_id, salt, result_hash)` – Commit phase
- `agora.revealResult(job_id, result)` – Reveal phase; pallet verifies salted hash
- `agora.finalizeJob(job_id)` – Finalizes by consensus (auto‑finalization also runs in `on_initialize`)
- `agora.requestRemoteJob(dest_para_id, input_data, bounty, reputation)` – Send cross‑para job via XCM

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

## Configuration and Parameters

Runtime (`runtime/src/lib.rs`) sets:
- Block time: 6s; weight/fee mapping
- `Agora` constants: `MinWorkerStake`, `MinJobBounty`, commit/reveal durations, limits
- XCM router: Parent UMP + XCMP; permissive `Barrier` and `Everything` filters for local testing

Harden for production:
- Restrict XCM execute/send filters; require paid execution with proper fee trading
- Tighten `LocationToAccountId` conversions and origins
- Replace simulated OCW HTTP with a secure, signed, and verifiable approach
