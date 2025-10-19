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

use alloc::{format, string::String, vec::Vec};
use frame::prelude::*;
use frame::traits::fungible::{Inspect, Mutate, MutateHold};
use types::*;

// --- CORRECTED IMPORTS FOR OCW SIGNED TRANSACTIONS ---
use ::frame_system::offchain::{
    AppCrypto, CreateSignedTransaction, Signer, SendSignedTransaction
};

use frame::deps::sp_io;
use frame_support::traits::Get;
use codec::{Encode, Decode};
// --- END CORRECTED IMPORTS ---

/// Key type for the Offchain Worker.
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"agor");

// Declare the application key type for sr25519
pub mod crypto {
    use super::KEY_TYPE;
    use sp_application_crypto::{app_crypto, sr25519};
    app_crypto!(sr25519, KEY_TYPE);
}

pub type AuthId = crypto::Public;
// --- END CORRECTED IMPORTS ---


#[frame::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The currency mechanism for this pallet
        // FIX: Fully qualify AccountId in trait bounds
        type Currency: Inspect<<Self as frame_system::Config>::AccountId, Balance = u128>
            + Mutate<<Self as frame_system::Config>::AccountId>
            + MutateHold<<Self as frame_system::Config>::AccountId, Reason = Self::RuntimeHoldReason>;

        /// The overarching hold reason
        type RuntimeHoldReason: From<HoldReason>;

        /// Weight information for extrinsics in this pallet
        // FIX: Fully qualify DbWeight using the full path where it is defined
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
        
         type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
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
    pub type Workers<T: Config> = StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::AccountId, WorkerInfo<T>>;

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
        <T as frame_system::Config>::AccountId,
        BlockNumberFor<T>,
        ValueQuery,
    >;

    /// Storage for tracking concurrent jobs per account
    #[pallet::storage]
    #[pallet::getter(fn account_job_count)]
    pub type AccountJobCount<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        <T as frame_system::Config>::AccountId,
        u32,
        ValueQuery,
    >;
    
    /// Local storage for OCW execution state
    #[pallet::storage]
    pub(super) type OCWState<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        JobId,
        OCWExecutionState<T>,
        OptionQuery,
    >;

    /// Events emitted by the pallet
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new job has been submitted
        JobSubmitted { job_id: JobId, creator: <T as frame_system::Config>::AccountId, bounty: u128 },
        /// A worker has been registered
        WorkerRegistered { worker: <T as frame_system::Config>::AccountId, stake: u128 },
        /// A worker has been unregistered
        WorkerUnregistered { worker: <T as frame_system::Config>::AccountId },
        /// A result has been committed
        ResultCommitted { job_id: JobId, worker: <T as frame_system::Config>::AccountId },
        /// A result has been revealed
        ResultRevealed { job_id: JobId, worker: <T as frame_system::Config>::AccountId },
        /// A job has been finalized
        JobFinalized { job_id: JobId, result: Vec<u8> },
        /// A worker has been rewarded
        WorkerRewarded { job_id: JobId, worker: <T as frame_system::Config>::AccountId, amount: u128 },
        /// A worker has been slashed
        WorkerSlashed { job_id: JobId, worker: <T as frame_system::Config>::AccountId, amount: u128 },
        /// OCW successfully submitted a signed transaction
        OCWTransactionSubmitted { job_id: JobId, is_reveal: bool },
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
        /// Offchain Worker failed to sign or submit transaction
        OCWSubmitError,
        /// Offchain Worker execution state not found
        OCWStateNotFound,
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
        log::info!("🔧 Agora OCW executing at block {:?}", block_number);
        
        // Process pending jobs
        Self::ocw_process_jobs(block_number);
        }
    }

    /// Defines which function the payload is for
    #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, scale_info::TypeInfo)]
    pub enum OcwFunction {
        Commit,
        Reveal,
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
        // FIX: Fully qualify DbWeight
        #[pallet::weight(Weight::from_parts(10_000, 0) + <T as frame_system::Config>::DbWeight::get().writes(2))]
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
        // FIX: Fully qualify DbWeight
        #[pallet::weight(Weight::from_parts(10_000, 0) + <T as frame_system::Config>::DbWeight::get().writes(1))]
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
        // FIX: Fully qualify DbWeight
        #[pallet::weight(Weight::from_parts(10_000, 0) + <T as frame_system::Config>::DbWeight::get().writes(1))]
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
        // FIX: Fully qualify DbWeight
        #[pallet::weight(Weight::from_parts(10_000, 0) + <T as frame_system::Config>::DbWeight::get().reads_writes(2, 1))]
        pub fn commit_result(
            origin: OriginFor<T>,
            job_id: JobId,
            salt: [u8; 32],
            // FIX: Fully qualify Hash
            result_hash: <T as frame_system::Config>::Hash,
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
                !commits.iter().any(|c| worker == c.worker),
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
        // FIX: Fully qualify DbWeight
        #[pallet::weight(Weight::from_parts(10_000, 0) + <T as frame_system::Config>::DbWeight::get().reads_writes(3, 2))]
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
                .find(|c| worker == c.worker)
                .ok_or(Error::<T>::NotCommitted)?;

            // Verify salted hash matches: hash(salt || result)
            let mut salted_input = Vec::new();
            salted_input.extend_from_slice(&commit.salt);
            salted_input.extend_from_slice(&result);
            // FIX: Fully qualify Hashing
            let salted_hash = <T as frame_system::Config>::Hashing::hash_of(&salted_input);
            // FIX: Use .as_ref() for comparison to handle the different types from `T::Hashing`
            ensure!(salted_hash.as_ref() == commit.result_hash.as_ref(), Error::<T>::SaltVerificationFailed);

            // Convert to bounded vec
            let bounded_result: BoundedVec<u8, ConstU32<2048>> =
                result.clone().try_into().map_err(|_| Error::<T>::InputDataTooLarge)?;

            // Get or create reveals vector
            let mut reveals = Reveals::<T>::get(job_id).unwrap_or_default();

            // Check if worker already revealed
            ensure!(
                !reveals.iter().any(|r| worker == r.worker),
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
        // FIX: Fully qualify DbWeight
        #[pallet::weight(Weight::from_parts(50_000, 0) + <T as frame_system::Config>::DbWeight::get().reads_writes(5, 5))]
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
        /// Off-chain worker job processing
        fn ocw_process_jobs(block_number: BlockNumberFor<T>) {
            log::info!("🔍 OCW checking for jobs at block {:?}", block_number);
            
            let pending_jobs = Self::get_pending_jobs();
            
            for job_id in pending_jobs {
                if Self::should_execute_job(job_id) {
                    if let Some(job) = Jobs::<T>::get(job_id) {
                        match job.job_type {
                            JobType::ApiRequest => {
                                Self::execute_api_job_with_commit(job_id, &job, block_number);
                            },
                            JobType::Computation => {
                                Self::execute_computation_job_with_commit(job_id, &job, block_number);
                            },
                        }
                    }
                }
            }
            
            let reveal_jobs = Self::get_jobs_ready_for_reveal(block_number);
            
            for job_id in reveal_jobs {
                Self::submit_reveal_transaction(job_id, block_number);
            }
        }
        
        /// Get all pending jobs
        fn get_pending_jobs() -> Vec<JobId> {
            let mut pending_jobs = Vec::new();
            
            Jobs::<T>::iter().for_each(|(job_id, job)| {
                // Also check if OCW has already stored the state for this job
                if job.status == JobStatus::Pending && !OCWState::<T>::contains_key(job_id) {
                    pending_jobs.push(job_id);
                }
            });
            
            pending_jobs
        }
        
        /// Check if job should be executed by OCW
        fn should_execute_job(_job_id: JobId) -> bool {
            true
        }
        
        /// Get jobs ready for reveal phase
        fn get_jobs_ready_for_reveal(current_block: BlockNumberFor<T>) -> Vec<JobId> {
            let mut reveal_jobs = Vec::new();
            
            Jobs::<T>::iter().for_each(|(job_id, job)| {
                if job.status == JobStatus::CommitPhase && current_block > job.commit_deadline {
                    if Self::has_pending_commit(job_id) {
                        reveal_jobs.push(job_id);
                    }
                }
            });
            
            reveal_jobs
        }
        
        /// Check if we have a pending commit for this job in local storage
        fn has_pending_commit(job_id: JobId) -> bool {
            OCWState::<T>::get(job_id).is_some()
        }
        
        /// Execute an API job with automated commit submission
        fn execute_api_job_with_commit(job_id: JobId, job: &Job<T>, block_number: BlockNumberFor<T>) {
            log::info!("🌐 Starting API execution for job {}", job_id);
            
            let input_str = String::from_utf8_lossy(&job.input_data);
            
            // For now, simulate API response
            let simulated_response = format!("API response for: {}", input_str);
            
            // Generate salt and commit hash
            let salt = Self::generate_salt();
            let result_bytes = simulated_response.as_bytes().to_vec();
            let commit_hash_bytes = Self::calculate_commit_hash_bytes(&salt, &result_bytes);
            
            // Store execution state
            let execution_state = OCWExecutionState {
                job_id,
                status: OCWJobStatus::ExecutionCompleted,
                result: BoundedVec::try_from(result_bytes).unwrap_or_default(),
                salt,
                commit_hash: H256(commit_hash_bytes),
                execution_start_block: block_number,
                commit_block: None,
                reveal_block: None,
            };
            
            Self::store_execution_state(job_id, &execution_state);
            
            // Submit commit transaction (SIGNED)
            Self::submit_commit_transaction_signed(job_id, salt, commit_hash_bytes, block_number);
        }
        
        /// Execute a computation job with automated commit submission
        fn execute_computation_job_with_commit(job_id: JobId, job: &Job<T>, block_number: BlockNumberFor<T>) {
            log::info!("🧮 Starting computation execution for job {}", job_id);
            
            // Simple computation: hash the input
            let result = frame::hashing::blake2_256(&job.input_data);
            
            // Generate salt and commit hash
            let salt = Self::generate_salt();
            let result_bytes = result.to_vec();
            let commit_hash_bytes = Self::calculate_commit_hash_bytes(&salt, &result_bytes);
            
            // Store execution state
            let execution_state = OCWExecutionState {
                job_id,
                status: OCWJobStatus::ExecutionCompleted,
                result: BoundedVec::try_from(result_bytes).unwrap_or_default(),
                salt,
                commit_hash: H256(commit_hash_bytes),
                execution_start_block: block_number,
                commit_block: None,
                reveal_block: None,
            };
            
            Self::store_execution_state(job_id, &execution_state);
            
            // Submit commit transaction (SIGNED)
            Self::submit_commit_transaction_signed(job_id, salt, commit_hash_bytes, block_number);
        }
        
        /// Submit a signed commit transaction
        fn submit_commit_transaction_signed(job_id: JobId, salt: [u8; 32], commit_hash: [u8; 32], block_number: BlockNumberFor<T>) {
            if let Err(e) = Self::submit_signed_transaction(job_id, OcwFunction::Commit, salt, commit_hash, BoundedVec::default(), block_number) {
                log::error!("Failed to submit signed commit transaction for job {}: {:?}", job_id, e);
            }
        }

        /// Submit a signed reveal transaction
        fn submit_reveal_transaction(job_id: JobId, block_number: BlockNumberFor<T>) {
            if let Some(state) = Self::get_execution_state(job_id) {
                // Get the raw hash bytes from the state (which is the frame_system::Config::Hash type)
                let commit_hash_bytes: [u8; 32] = state.commit_hash.as_ref().try_into().unwrap_or([0; 32]);

                if let Err(e) = Self::submit_signed_transaction(
                    job_id, 
                    OcwFunction::Reveal, 
                    state.salt, 
                    commit_hash_bytes, 
                    state.result, 
                    block_number
                ) {
                    log::error!("Failed to submit signed reveal transaction for job {}: {:?}", job_id, e);
                }
            } else {
                log::error!("❌ No execution state found for job {}", job_id);
            }
        }
        
        /// Generic function to submit a signed transaction for OCW
		fn submit_signed_transaction(
			job_id: JobId,
			function: OcwFunction,
			salt: [u8; 32],
			commit_hash: [u8; 32],
			result: BoundedVec<u8, ConstU32<2048>>,
			_block_number: BlockNumberFor<T>,
		) -> Result<(), Error<T>> {
			// Use the sr25519 public key type directly
			let signer = Signer::<T, T::AuthorityId>::any_account();
			
			if !signer.can_sign() {
				log::error!("No local accounts configured for signing");
				return Err(Error::<T>::OCWSubmitError);
			}
			
			let results = signer.send_signed_transaction(|_account| {
				match &function {
					OcwFunction::Commit => {
						Call::commit_result {
							job_id,
							salt,
							result_hash: <T as frame_system::Config>::Hashing::hash(&commit_hash),
						}
					},
					OcwFunction::Reveal => {
						Call::reveal_result {
							job_id,
							result: result.to_vec(),
						}
					},
				}
			});
			
			for (_, res) in results {
				match res {
					Ok(()) => {
						Self::deposit_event(Event::OCWTransactionSubmitted { 
							job_id, 
							is_reveal: matches!(function, OcwFunction::Reveal),
						});
						return Ok(());
					},
					Err(e) => {
						log::error!("Error submitting signed transaction: {:?}", e);
					}
				}
			}
			
			Err(Error::<T>::OCWSubmitError)
		}

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

                    let worker_account_id: <T as frame_system::Config>::AccountId = worker;

                    Self::deposit_event(Event::WorkerRewarded {
                        job_id,
                        worker: worker_account_id,
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
                    
                    let worker_account_id: <T as frame_system::Config>::AccountId = worker;

                    Self::deposit_event(Event::WorkerSlashed {
                        job_id,
                        worker: worker_account_id,
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
                            // FIX: Fully qualify DbWeight
                            weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(1));
                        }
                    }
                    JobStatus::CommitPhase => {
                        // Auto-transition to RevealPhase when commit deadline passes
                        if block_number > job.commit_deadline {
                            job.status = JobStatus::RevealPhase;
                            Jobs::<T>::insert(job_id, job);
                            processed_jobs += 1;
                            // FIX: Fully qualify DbWeight
                            weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(1));
                        }
                    }
                    JobStatus::RevealPhase => {
                        // Auto-finalize when reveal deadline passes
                        if block_number > job.reveal_deadline {
                            if let Ok(_) = Self::finalize_job_internal(job_id, &job) {
                                processed_jobs += 1;
                                // FIX: Fully qualify DbWeight
                                weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().reads_writes(5, 3));
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
                        // FIX: Fully qualify DbWeight
                        weight = weight.saturating_add(<T as frame_system::Config>::DbWeight::get().writes(2));

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
        
        /// Generate a random salt for commit-reveal
        fn generate_salt() -> [u8; 32] {
            let mut salt = [0u8; 32];
            // FIX: Remove .ok() from the right side of the assignment since sp_io::offchain::random_seed() returns Result<[u8; 32], _>
            let random_seed = sp_io::offchain::random_seed();
            salt.copy_from_slice(&random_seed);
            salt
        }
        
        /// Calculate commit hash from salt and result
        fn calculate_commit_hash_bytes(salt: &[u8; 32], result: &[u8]) -> [u8; 32] {
            let mut salted_input = Vec::new();
            salted_input.extend_from_slice(salt);
            salted_input.extend_from_slice(result);
            frame::hashing::blake2_256(&salted_input)
        }
        
        /// Store execution state in local storage (OCWState)
        fn store_execution_state(job_id: JobId, state: &OCWExecutionState<T>) {
            OCWState::<T>::insert(job_id, state);
            log::info!("💾 Stored execution state for job {}", job_id);
        }
        
        /// Get execution state from local storage (OCWState)
        fn get_execution_state(job_id: JobId) -> Option<OCWExecutionState<T>> {
            OCWState::<T>::get(job_id)
        }
    }
}

/// Weight functions trait
pub trait WeightInfo {
    // FIX: DbWeight is now an associated type with a correct bound
    fn submit_job() -> Weight;
    fn register_worker() -> Weight;
    fn unregister_worker() -> Weight;
    fn commit_result() -> Weight;
    fn reveal_result() -> Weight;
    fn finalize_job() -> Weight;
}

/// Default weight implementation
impl WeightInfo for () {
    // FIX: Using sp_weights::RuntimeDbWeight as a concrete type that implements Get<Weight>
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