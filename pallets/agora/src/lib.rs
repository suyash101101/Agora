//! # Agora Pallet
//!
//! A verifiable computation marketplace pallet that enables off-chain workers to execute jobs
//! and reach consensus through a commit-reveal mechanism with crypto-economic incentives.
//!
//! ## Overview
//!
//! This pallet provides:
//! - Job submission with bounty locking
//! - Worker registration with staking
//! - Commit-reveal consensus mechanism
//! - Reward distribution and slashing
//! - Off-chain worker integration for job execution

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;
pub mod types;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use alloc::vec::Vec;
use frame::prelude::*;
use frame::traits::fungible::{Inspect, Mutate, MutateHold};
use types::*;

#[frame::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency mechanism for this pallet
		type Currency: Inspect<Self::AccountId, Balance = u128>
			+ Mutate<Self::AccountId>
			+ MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason>;

		/// The overarching hold reason
		type RuntimeHoldReason: From<HoldReason>;

		/// Weight information for extrinsics in this pallet
		type WeightInfo: WeightInfo;

		/// Duration of the commit phase in blocks
		#[pallet::constant]
		type CommitPhaseDuration: Get<BlockNumberFor<Self>>;

		/// Duration of the reveal phase in blocks
		#[pallet::constant]
		type RevealPhaseDuration: Get<BlockNumberFor<Self>>;

		/// Minimum stake required to register as a worker
		#[pallet::constant]
		type MinWorkerStake: Get<u128>;

		/// Minimum bounty for a job
		#[pallet::constant]
		type MinJobBounty: Get<u128>;

		/// Maximum input data size for a job
		#[pallet::constant]
		type MaxInputBytes: Get<u32>;

		/// Maximum number of commits per job
		#[pallet::constant]
		type MaxCommitsPerJob: Get<u32>;

		/// Maximum number of reveals per job
		#[pallet::constant]
		type MaxRevealsPerJob: Get<u32>;

		/// Maximum concurrent jobs per account
		#[pallet::constant]
		type MaxConcurrentJobsPerAccount: Get<u32>;

		/// Unbonding delay for workers in blocks
		#[pallet::constant]
		type UnbondingBlocks: Get<BlockNumberFor<Self>>;
	}

	/// Reasons for holding balances
	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Funds held for job bounty
		JobBounty,
		/// Funds held for worker stake
		WorkerStake,
	}

	/// Storage for jobs indexed by JobId
	#[pallet::storage]
	#[pallet::getter(fn jobs)]
	pub type Jobs<T: Config> = StorageMap<_, Blake2_128Concat, JobId, Job<T>>;

	/// Storage for workers indexed by AccountId
	#[pallet::storage]
	#[pallet::getter(fn workers)]
	pub type Workers<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, WorkerInfo<T>>;

	/// Storage for commits indexed by JobId
	#[pallet::storage]
	#[pallet::getter(fn commits)]
	pub type Commits<T: Config> =
		StorageMap<_, Blake2_128Concat, JobId, BoundedVec<Commit<T>, ConstU32<100>>>;

	/// Storage for reveals indexed by JobId
	#[pallet::storage]
	#[pallet::getter(fn reveals)]
	pub type Reveals<T: Config> =
		StorageMap<_, Blake2_128Concat, JobId, BoundedVec<Reveal<T>, ConstU32<100>>>;

	/// Storage for final results indexed by JobId
	#[pallet::storage]
	#[pallet::getter(fn results)]
	pub type Results<T: Config> = StorageMap<_, Blake2_128Concat, JobId, BoundedVec<u8, ConstU32<2048>>>;

	/// Counter for generating unique JobIds
	#[pallet::storage]
	#[pallet::getter(fn next_job_id)]
	pub type NextJobId<T: Config> = StorageValue<_, JobId, ValueQuery>;

	/// Storage for worker unbonding information
	#[pallet::storage]
	#[pallet::getter(fn unbonding_workers)]
	pub type UnbondingWorkers<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BlockNumberFor<T>,
		ValueQuery,
	>;

	/// Storage for tracking concurrent jobs per account
	#[pallet::storage]
	#[pallet::getter(fn account_job_count)]
	pub type AccountJobCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		u32,
		ValueQuery,
	>;

	/// Events emitted by the pallet
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new job has been submitted
		JobSubmitted { job_id: JobId, creator: T::AccountId, bounty: u128 },
		/// A worker has been registered
		WorkerRegistered { worker: T::AccountId, stake: u128 },
		/// A worker has been unregistered
		WorkerUnregistered { worker: T::AccountId },
		/// A result has been committed
		ResultCommitted { job_id: JobId, worker: T::AccountId },
		/// A result has been revealed
		ResultRevealed { job_id: JobId, worker: T::AccountId },
		/// A job has been finalized
		JobFinalized { job_id: JobId, result: Vec<u8> },
		/// A worker has been rewarded
		WorkerRewarded { job_id: JobId, worker: T::AccountId, amount: u128 },
		/// A worker has been slashed
		WorkerSlashed { job_id: JobId, worker: T::AccountId, amount: u128 },
	}

	/// Errors that can be returned by the pallet
	#[pallet::error]
	pub enum Error<T> {
		/// Job does not exist
		JobNotFound,
		/// Worker is not registered
		WorkerNotRegistered,
		/// Worker is already registered
		WorkerAlreadyRegistered,
		/// Insufficient stake
		InsufficientStake,
		/// Insufficient bounty
		InsufficientBounty,
		/// Job is not in the correct phase
		InvalidJobPhase,
		/// Commit hash does not match revealed result
		CommitMismatch,
		/// Worker has already committed for this job
		AlreadyCommitted,
		/// Worker has not committed for this job
		NotCommitted,
		/// Commit deadline has passed
		CommitDeadlinePassed,
		/// Reveal deadline has passed
		RevealDeadlinePassed,
		/// Job has already been finalized
		JobAlreadyFinalized,
		/// Not enough reveals to finalize
		InsufficientReveals,
		/// Input data too large
		InputDataTooLarge,
		/// Insufficient balance
		InsufficientBalance,
		/// Worker is in unbonding period
		WorkerUnbonding,
		/// Too many concurrent jobs
		TooManyConcurrentJobs,
		/// Salt verification failed
		SaltVerificationFailed,
		/// Worker has already revealed for this job
		AlreadyRevealed,
		/// Job has been cancelled
		JobCancelled,
		/// Unbonding period not completed
		UnbondingPeriodNotCompleted,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Initialize block - handle job lifecycle transitions
		fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
			let mut weight = Weight::from_parts(0, 0);
			
			// Process job lifecycle transitions
			weight = weight.saturating_add(Self::process_job_transitions(block_number));
			
			// Process unbonding workers
			weight = weight.saturating_add(Self::process_unbonding_workers(block_number));
			
			weight
		}

		/// Off-chain worker entry point
		fn offchain_worker(block_number: BlockNumberFor<T>) {
			log::info!("🔧 Agora off-chain worker executing at block {:?}", block_number);
			
			// This is a placeholder for off-chain worker logic
			// In a full implementation, this would:
			// 1. Fetch pending jobs from storage
			// 2. Execute jobs (API calls, computations)
			// 3. Submit commit transactions
			// 4. Submit reveal transactions when appropriate
		}
	}

	/// Validate unsigned transactions from off-chain workers
	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// For now, reject all unsigned transactions
			// In a full implementation, this would validate OCW transactions
			InvalidTransaction::Call.into()
		}
	}

	/// Dispatchable functions (extrinsics)
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit a new job with a bounty
		///
		/// # Arguments
		/// * `origin` - The account submitting the job
		/// * `job_type_id` - Type of job (0 = ApiRequest, 1 = Computation)
		/// * `input_data` - Input data for the job
		/// * `bounty` - Bounty amount to lock
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(2))]
		pub fn submit_job(
			origin: OriginFor<T>,
			job_type_id: u8,
			input_data: Vec<u8>,
			bounty: u128,
		) -> DispatchResult {
			let creator = ensure_signed(origin)?;

			// Validate bounty
			ensure!(bounty >= T::MinJobBounty::get(), Error::<T>::InsufficientBounty);

			// Convert job_type_id to JobType
			let job_type = match job_type_id {
				0 => JobType::ApiRequest,
				1 => JobType::Computation,
				_ => return Err(Error::<T>::InvalidJobPhase.into()),
			};

			// Validate input data size
			let bounded_input: BoundedVec<u8, ConstU32<1024>> =
				input_data.try_into().map_err(|_| Error::<T>::InputDataTooLarge)?;

			// Check balance
			let balance = T::Currency::balance(&creator);
			ensure!(balance >= bounty, Error::<T>::InsufficientBalance);

			// Lock bounty
			T::Currency::hold(&HoldReason::JobBounty.into(), &creator, bounty)?;

			// Generate job ID
			let job_id = NextJobId::<T>::get();
			NextJobId::<T>::put(job_id.saturating_add(1));

			// Get current block number
			let current_block = frame_system::Pallet::<T>::block_number();
			let commit_deadline = current_block + T::CommitPhaseDuration::get();
			let reveal_deadline = commit_deadline + T::RevealPhaseDuration::get();

			// Create job
			let job = Job {
				creator: creator.clone(),
				bounty,
				job_type,
				input_data: bounded_input,
				status: JobStatus::Pending,
				created_at: current_block,
				commit_deadline,
				reveal_deadline,
			};

			// Store job
			Jobs::<T>::insert(job_id, job);

			// Emit event
			Self::deposit_event(Event::JobSubmitted { job_id, creator, bounty });

			Ok(())
		}

		/// Register as a worker with a stake
		///
		/// # Arguments
		/// * `origin` - The account registering as a worker
		/// * `stake` - Amount to stake
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn register_worker(origin: OriginFor<T>, stake: u128) -> DispatchResult {
			let worker = ensure_signed(origin)?;

			// Check if already registered
			ensure!(!Workers::<T>::contains_key(&worker), Error::<T>::WorkerAlreadyRegistered);

			// Validate stake
			ensure!(stake >= T::MinWorkerStake::get(), Error::<T>::InsufficientStake);

			// Check balance
			let balance = T::Currency::balance(&worker);
			ensure!(balance >= stake, Error::<T>::InsufficientBalance);

			// Lock stake
			T::Currency::hold(&HoldReason::WorkerStake.into(), &worker, stake)?;

			// Create worker info
			let worker_info = WorkerInfo {
				stake,
				reputation: 500, // Start with neutral reputation
				is_active: true,
				registered_at: frame_system::Pallet::<T>::block_number(),
			};

			// Store worker
			Workers::<T>::insert(&worker, worker_info);

			// Emit event
			Self::deposit_event(Event::WorkerRegistered { worker, stake });

			Ok(())
		}

		/// Unregister as a worker and return stake
		///
		/// # Arguments
		/// * `origin` - The worker account to unregister
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn unregister_worker(origin: OriginFor<T>) -> DispatchResult {
			let worker = ensure_signed(origin)?;

			// Get worker info
			let worker_info = Workers::<T>::get(&worker).ok_or(Error::<T>::WorkerNotRegistered)?;

			// Release stake
			T::Currency::release(&HoldReason::WorkerStake.into(), &worker, worker_info.stake, frame::traits::tokens::Precision::Exact)?;

			// Remove worker
			Workers::<T>::remove(&worker);

			// Emit event
			Self::deposit_event(Event::WorkerUnregistered { worker });

			Ok(())
		}

		/// Commit a result hash for a job
		///
		/// # Arguments
		/// * `origin` - The worker committing the result
		/// * `job_id` - ID of the job
		/// * `salt` - Salt used for hashing (32 bytes)
		/// * `result_hash` - Hash of salt + result
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(2, 1))]
		pub fn commit_result(
			origin: OriginFor<T>,
			job_id: JobId,
			salt: [u8; 32],
			result_hash: T::Hash,
		) -> DispatchResult {
			let worker = ensure_signed(origin)?;

			// Verify worker is registered
			ensure!(Workers::<T>::contains_key(&worker), Error::<T>::WorkerNotRegistered);

			// Get job
			let mut job = Jobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;

			// Check job phase
			let current_block = frame_system::Pallet::<T>::block_number();
			ensure!(current_block <= job.commit_deadline, Error::<T>::CommitDeadlinePassed);

			// Update job status to CommitPhase if still Pending
			if job.status == JobStatus::Pending {
				job.status = JobStatus::CommitPhase;
				Jobs::<T>::insert(job_id, job);
			}

			// Get or create commits vector
			let mut commits = Commits::<T>::get(job_id).unwrap_or_default();

			// Check if worker already committed
			ensure!(
				!commits.iter().any(|c| c.worker == worker),
				Error::<T>::AlreadyCommitted
			);

			// Create commit
			let commit = Commit { 
				worker: worker.clone(), 
				salt,
				result_hash, 
				committed_at: current_block 
			};

			// Add commit
			commits.try_push(commit).map_err(|_| Error::<T>::AlreadyCommitted)?;
			Commits::<T>::insert(job_id, commits);

			// Emit event
			Self::deposit_event(Event::ResultCommitted { job_id, worker });

			Ok(())
		}

		/// Reveal a result for a job
		///
		/// # Arguments
		/// * `origin` - The worker revealing the result
		/// * `job_id` - ID of the job
		/// * `result` - The actual result data
		#[pallet::call_index(4)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().reads_writes(3, 2))]
		pub fn reveal_result(
			origin: OriginFor<T>,
			job_id: JobId,
			result: Vec<u8>,
		) -> DispatchResult {
			let worker = ensure_signed(origin)?;

			// Get job
			let mut job = Jobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;

			// Check phase timing
			let current_block = frame_system::Pallet::<T>::block_number();
			ensure!(current_block > job.commit_deadline, Error::<T>::InvalidJobPhase);
			ensure!(current_block <= job.reveal_deadline, Error::<T>::RevealDeadlinePassed);

			// Update job status to RevealPhase if still in CommitPhase
			if job.status == JobStatus::CommitPhase {
				job.status = JobStatus::RevealPhase;
				Jobs::<T>::insert(job_id, &job);
			}

			// Get commits
			let commits = Commits::<T>::get(job_id).ok_or(Error::<T>::NotCommitted)?;

			// Find worker's commit
			let commit = commits
				.iter()
				.find(|c| c.worker == worker)
				.ok_or(Error::<T>::NotCommitted)?;

			// Verify salted hash matches: hash(salt || result)
			let mut salted_input = Vec::new();
			salted_input.extend_from_slice(&commit.salt);
			salted_input.extend_from_slice(&result);
			let salted_hash = T::Hashing::hash(&salted_input);
			ensure!(salted_hash == commit.result_hash, Error::<T>::SaltVerificationFailed);

			// Convert to bounded vec
			let bounded_result: BoundedVec<u8, ConstU32<2048>> =
				result.clone().try_into().map_err(|_| Error::<T>::InputDataTooLarge)?;

			// Get or create reveals vector
			let mut reveals = Reveals::<T>::get(job_id).unwrap_or_default();

			// Check if worker already revealed
			ensure!(
				!reveals.iter().any(|r| r.worker == worker),
				Error::<T>::AlreadyRevealed
			);

			// Create reveal
			let reveal = Reveal {
				worker: worker.clone(),
				salt: commit.salt,
				result: bounded_result,
				revealed_at: current_block,
			};

			// Add reveal
			reveals.try_push(reveal).map_err(|_| Error::<T>::AlreadyCommitted)?;
			Reveals::<T>::insert(job_id, reveals);

			// Emit event
			Self::deposit_event(Event::ResultRevealed { job_id, worker });

			Ok(())
		}

		/// Finalize a job by determining consensus and distributing rewards
		///
		/// # Arguments
		/// * `origin` - Any signed account can finalize
		/// * `job_id` - ID of the job to finalize
		#[pallet::call_index(5)]
		#[pallet::weight(Weight::from_parts(50_000, 0) + T::DbWeight::get().reads_writes(5, 5))]
		pub fn finalize_job(origin: OriginFor<T>, job_id: JobId) -> DispatchResult {
			let _caller = ensure_signed(origin)?;

			// Get job
			let mut job = Jobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;

			// Check job not already finalized
			ensure!(job.status != JobStatus::Completed, Error::<T>::JobAlreadyFinalized);

			// Check reveal phase has ended
			let current_block = frame_system::Pallet::<T>::block_number();
			ensure!(current_block > job.reveal_deadline, Error::<T>::InvalidJobPhase);

			// Get reveals
			let reveals = Reveals::<T>::get(job_id).ok_or(Error::<T>::InsufficientReveals)?;
			ensure!(!reveals.is_empty(), Error::<T>::InsufficientReveals);

			// Determine consensus result (simple majority)
			let consensus_result = Self::determine_consensus(&reveals)?;

			// Store final result
			Results::<T>::insert(job_id, consensus_result.clone());

			// Distribute rewards and slash dishonest workers
			Self::distribute_rewards_and_slash(job_id, &job, &reveals, &consensus_result)?;

			// Update job status
			job.status = JobStatus::Completed;
			Jobs::<T>::insert(job_id, job);

			// Emit event
			Self::deposit_event(Event::JobFinalized {
				job_id,
				result: consensus_result.to_vec(),
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Determine consensus result from reveals (simple majority)
		fn determine_consensus(
			reveals: &BoundedVec<Reveal<T>, ConstU32<100>>,
		) -> Result<BoundedVec<u8, ConstU32<2048>>, DispatchError> {
			use alloc::collections::BTreeMap;

			// Count occurrences of each result
			let mut result_counts: BTreeMap<Vec<u8>, usize> = BTreeMap::new();

			for reveal in reveals.iter() {
				let result_vec = reveal.result.to_vec();
				*result_counts.entry(result_vec).or_insert(0) += 1;
			}

			// Find result with most votes
			let consensus = result_counts
				.into_iter()
				.max_by_key(|(_, count)| *count)
				.map(|(result, _)| result)
				.ok_or(Error::<T>::InsufficientReveals)?;

			// Convert to bounded vec
			consensus.try_into().map_err(|_| Error::<T>::InputDataTooLarge.into())
		}

		/// Distribute rewards to honest workers and slash dishonest ones
		fn distribute_rewards_and_slash(
			job_id: JobId,
			job: &Job<T>,
			reveals: &BoundedVec<Reveal<T>, ConstU32<100>>,
			consensus_result: &BoundedVec<u8, ConstU32<2048>>,
		) -> DispatchResult {
			let honest_workers: Vec<_> = reveals
				.iter()
				.filter(|r| &r.result == consensus_result)
				.map(|r| r.worker.clone())
				.collect();

			let dishonest_workers: Vec<_> = reveals
				.iter()
				.filter(|r| &r.result != consensus_result)
				.map(|r| r.worker.clone())
				.collect();

			// Calculate reward per honest worker
			let total_honest = honest_workers.len() as u128;
			if total_honest > 0 {
				let reward_per_worker = job.bounty / total_honest;

				// Release bounty and distribute to honest workers
				T::Currency::release(&HoldReason::JobBounty.into(), &job.creator, job.bounty, frame::traits::tokens::Precision::BestEffort)?;

				for worker in honest_workers {
					// Transfer reward
					T::Currency::transfer(&job.creator, &worker, reward_per_worker, frame::traits::tokens::Preservation::Preserve)?;

					// Update reputation
					if let Some(mut worker_info) = Workers::<T>::get(&worker) {
						worker_info.reputation = worker_info.reputation.saturating_add(10).min(1000);
						Workers::<T>::insert(&worker, worker_info);
					}

					Self::deposit_event(Event::WorkerRewarded {
						job_id,
						worker,
						amount: reward_per_worker,
					});
				}
			}

			// Slash dishonest workers
			for worker in dishonest_workers {
				if let Some(mut worker_info) = Workers::<T>::get(&worker) {
					// Slash 10% of stake
					let slash_amount = worker_info.stake / 10;
					
					// Reduce stake
					worker_info.stake = worker_info.stake.saturating_sub(slash_amount);
					
					// Reduce reputation
					worker_info.reputation = worker_info.reputation.saturating_sub(50);
					
					Workers::<T>::insert(&worker, worker_info);

					Self::deposit_event(Event::WorkerSlashed {
						job_id,
						worker,
						amount: slash_amount,
					});
				}
			}

			Ok(())
		}

		/// Process job lifecycle transitions (called in on_initialize)
		fn process_job_transitions(block_number: BlockNumberFor<T>) -> Weight {
			let mut weight = Weight::from_parts(0, 0);
			let mut processed_jobs = 0;

			// Iterate through all jobs to find those needing transitions
			Jobs::<T>::iter().for_each(|(job_id, mut job)| {
				if processed_jobs >= 10 { // Limit processing per block
					return;
				}

				match job.status {
					JobStatus::Pending => {
						// Auto-transition to CommitPhase when commit deadline approaches
						if block_number >= job.commit_deadline.saturating_sub(T::CommitPhaseDuration::get()) {
							job.status = JobStatus::CommitPhase;
							Jobs::<T>::insert(job_id, job);
							processed_jobs += 1;
							weight = weight.saturating_add(T::DbWeight::get().writes(1));
						}
					}
					JobStatus::CommitPhase => {
						// Auto-transition to RevealPhase when commit deadline passes
						if block_number > job.commit_deadline {
							job.status = JobStatus::RevealPhase;
							Jobs::<T>::insert(job_id, job);
							processed_jobs += 1;
							weight = weight.saturating_add(T::DbWeight::get().writes(1));
						}
					}
					JobStatus::RevealPhase => {
						// Auto-finalize when reveal deadline passes
						if block_number > job.reveal_deadline {
							if let Ok(_) = Self::finalize_job_internal(job_id, &job) {
								processed_jobs += 1;
								weight = weight.saturating_add(T::DbWeight::get().reads_writes(5, 3));
							}
						}
					}
					_ => {} // No action needed for Completed/Failed jobs
				}
			});

			weight
		}

		/// Process unbonding workers (called in on_initialize)
		fn process_unbonding_workers(block_number: BlockNumberFor<T>) -> Weight {
			let mut weight = Weight::from_parts(0, 0);
			let mut processed_workers = 0;

			// Find workers ready to complete unbonding
			UnbondingWorkers::<T>::iter().for_each(|(worker, unbonding_block)| {
				if processed_workers >= 5 { // Limit processing per block
					return;
				}

				if block_number >= unbonding_block {
					// Complete unbonding
					if let Some(worker_info) = Workers::<T>::get(&worker) {
						// Release stake
						let _ = T::Currency::release(
							&HoldReason::WorkerStake.into(),
							&worker,
							worker_info.stake,
							frame::traits::tokens::Precision::BestEffort,
						);

						// Remove from unbonding and workers
						UnbondingWorkers::<T>::remove(&worker);
						Workers::<T>::remove(&worker);

						processed_workers += 1;
						weight = weight.saturating_add(T::DbWeight::get().writes(2));

						Self::deposit_event(Event::WorkerUnregistered { worker });
					}
				}
			});

			weight
		}

		/// Internal finalize job function (used by both manual and auto-finalization)
		fn finalize_job_internal(job_id: JobId, job: &Job<T>) -> DispatchResult {
			// Get reveals
			let reveals = Reveals::<T>::get(job_id).ok_or(Error::<T>::InsufficientReveals)?;
			ensure!(reveals.len() > 0, Error::<T>::InsufficientReveals);

			// Determine consensus result
			let consensus_result = Self::determine_consensus(&reveals)?;

			// Distribute rewards and slash dishonest workers
			Self::distribute_rewards_and_slash(job_id, job, &reveals, &consensus_result)?;

			// Store final result
			Results::<T>::insert(job_id, consensus_result.clone());

			// Update job status
			let mut updated_job = job.clone();
			updated_job.status = JobStatus::Completed;
			Jobs::<T>::insert(job_id, updated_job);

			// Emit event
			Self::deposit_event(Event::JobFinalized {
				job_id,
				result: consensus_result.to_vec(),
			});

			Ok(())
		}
	}
}

/// Weight functions trait
pub trait WeightInfo {
	fn submit_job() -> Weight;
	fn register_worker() -> Weight;
	fn unregister_worker() -> Weight;
	fn commit_result() -> Weight;
	fn reveal_result() -> Weight;
	fn finalize_job() -> Weight;
}

/// Default weight implementation
impl WeightInfo for () {
	fn submit_job() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn register_worker() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn unregister_worker() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn commit_result() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn reveal_result() -> Weight {
		Weight::from_parts(10_000, 0)
	}
	fn finalize_job() -> Weight {
		Weight::from_parts(50_000, 0)
	}
}

