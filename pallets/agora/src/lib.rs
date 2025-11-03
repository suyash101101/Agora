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
mod functions;
mod ocw;

pub mod xcm_job_client;
pub mod xcm_handler;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use alloc::vec::Vec;
use frame::prelude::*;
use frame::traits::fungible::{Inspect, Mutate, MutateHold};
use types::*;
use polkadot_sdk::cumulus_primitives_core::ParaId;

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
        type Currency: Inspect<<Self as frame_system::Config>::AccountId, Balance = u128>
            + Mutate<<Self as frame_system::Config>::AccountId>
            + MutateHold<<Self as frame_system::Config>::AccountId, Reason = Self::RuntimeHoldReason>;

        /// The overarching hold reason
        type RuntimeHoldReason: From<HoldReason>;

        /// Weight information for extrinsics in this pallet
        type WeightInfo: WeightInfo;

        type RuntimeCall: From<Call<Self>> + Encode;
        
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

         #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// XCM sender for cross-chain communication
        type XcmSender: staging_xcm::prelude::SendXcm;

        type ParaId: Get<ParaId>;
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

    #[pallet::storage]
    #[pallet::getter(fn pending_jobs)]
    pub type PendingJobs<T: Config> = StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::Hash, (T::AccountId, u32, u128)>;

    /// Nonce for generating unique job IDs for XCM
    #[pallet::storage]
    #[pallet::getter(fn job_nonce)]
    pub type JobNonce<T: Config> = StorageValue<_, u64, ValueQuery>;

    #[pallet::storage]
    pub type RemoteJobInfo<T: Config> = StorageMap<_, Blake2_128Concat, JobId, (u32, <T as frame_system::Config>::Hash)>;

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
        /// A job has been submitted via XCM from another parachain
        XcmJobSubmitted { job_id: JobId, creator: <T as frame_system::Config>::AccountId, bounty: u128, origin_para_id: u32 },
        /// A job result has been queried (can be used by XCM response handlers)
        JobResultQueried { job_id: JobId, result: Vec<u8>, origin_para_id: u32 },
        /// XCM Job Client events
        /// Remote job request sent via XCM
        RemoteJobRequested {
            job_id: <T as frame_system::Config>::Hash,
            sender: <T as frame_system::Config>::AccountId,
            dest_para_id: u32,
            bounty: u128,
        },
        /// Remote job completed and result received
        RemoteJobCompleted {
            job_id: <T as frame_system::Config>::Hash,
            result_hash: <T as frame_system::Config>::Hash,
        },
        /// Remote job failed
        RemoteJobFailed {
            job_id: <T as frame_system::Config>::Hash,
            reason: Vec<u8>,
        },
        /// XCM message sent successfully
        XcmMessageSent {
            destination: u32,
            message_hash: <T as frame_system::Config>::Hash,
        },
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
        /// XCM send failed
        XcmSendFailed,
        Overflow,
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
            
            // Delegate to OCW module
            Self::ocw_process_jobs(block_number);
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
                origin_para_id: 0,
                result: BoundedVec::default(),
            };

            // Store job
            Jobs::<T>::insert(job_id, job);

            // Emit event
            Self::deposit_event(Event::JobSubmitted { job_id, creator, bounty });

            Ok(())
        }

        /// Register as a worker with a stake
        #[pallet::call_index(1)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + <T as frame_system::Config>::DbWeight::get().writes(1))]
        pub fn register_worker(origin: OriginFor<T>, stake: u128) -> DispatchResult {
            let worker = ensure_signed(origin)?;

            ensure!(!Workers::<T>::contains_key(&worker), Error::<T>::WorkerAlreadyRegistered);
            ensure!(stake >= T::MinWorkerStake::get(), Error::<T>::InsufficientStake);

            let balance = T::Currency::balance(&worker);
            ensure!(balance >= stake, Error::<T>::InsufficientBalance);

            T::Currency::hold(&HoldReason::WorkerStake.into(), &worker, stake)?;

            let worker_info = WorkerInfo {
                stake,
                reputation: 500,
                is_active: true,
                registered_at: frame_system::Pallet::<T>::block_number(),
            };

            Workers::<T>::insert(&worker, worker_info);
            Self::deposit_event(Event::WorkerRegistered { worker, stake });

            Ok(())
        }

        /// Unregister as a worker and return stake
        #[pallet::call_index(2)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + <T as frame_system::Config>::DbWeight::get().writes(1))]
        pub fn unregister_worker(origin: OriginFor<T>) -> DispatchResult {
            let worker = ensure_signed(origin)?;
            let worker_info = Workers::<T>::get(&worker).ok_or(Error::<T>::WorkerNotRegistered)?;

            T::Currency::release(&HoldReason::WorkerStake.into(), &worker, worker_info.stake, frame::traits::tokens::Precision::Exact)?;
            Workers::<T>::remove(&worker);
            Self::deposit_event(Event::WorkerUnregistered { worker });

            Ok(())
        }

        /// Commit a result hash for a job
        #[pallet::call_index(3)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + <T as frame_system::Config>::DbWeight::get().reads_writes(2, 1))]
        pub fn commit_result(
            origin: OriginFor<T>,
            job_id: JobId,
            salt: [u8; 32],
            result_hash: <T as frame_system::Config>::Hash,
        ) -> DispatchResult {
            let worker = ensure_signed(origin)?;

            ensure!(Workers::<T>::contains_key(&worker), Error::<T>::WorkerNotRegistered);

            let mut job = Jobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;
            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(current_block <= job.commit_deadline, Error::<T>::CommitDeadlinePassed);

            if job.status == JobStatus::Pending {
                job.status = JobStatus::CommitPhase;
                Jobs::<T>::insert(job_id, job);
            }

            let mut commits = Commits::<T>::get(job_id).unwrap_or_default();
            ensure!(!commits.iter().any(|c| worker == c.worker), Error::<T>::AlreadyCommitted);

            let commit = Commit { 
                worker: worker.clone(), 
                salt,
                result_hash, 
                committed_at: current_block 
            };

            log::info!("📝 Worker {:?} committing result for job {}", worker, job_id);
            log::info!("🔒 Commit hash: {:?}", result_hash);
            log::info!("🧂 Salt: {:?}", salt);

            commits.try_push(commit).map_err(|_| Error::<T>::AlreadyCommitted)?;
            Commits::<T>::insert(job_id, commits);
            Self::deposit_event(Event::ResultCommitted { job_id, worker });

            Ok(())
        }

        /// Reveal a result for a job
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::from_parts(10_000, 0) + <T as frame_system::Config>::DbWeight::get().reads_writes(3, 2))]
        pub fn reveal_result(
            origin: OriginFor<T>,
            job_id: JobId,
            result: Vec<u8>,
        ) -> DispatchResult {
            let worker = ensure_signed(origin)?;

            let mut job = Jobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;
            let current_block = frame_system::Pallet::<T>::block_number();
            
            ensure!(current_block > job.commit_deadline, Error::<T>::InvalidJobPhase);
            ensure!(current_block <= job.reveal_deadline, Error::<T>::RevealDeadlinePassed);

            if job.status == JobStatus::CommitPhase {
                job.status = JobStatus::RevealPhase;
                Jobs::<T>::insert(job_id, &job);
            }

            let commits = Commits::<T>::get(job_id).ok_or(Error::<T>::NotCommitted)?;
            let commit = commits.iter().find(|c| worker == c.worker).ok_or(Error::<T>::NotCommitted)?;

            log::info!("🔓 Worker {:?} revealing result for job {}", worker, job_id);
            log::info!("🧂 Salt: {:?}", commit.salt);
            log::info!("📄 Result: {:?}", result);
            // Verify salted hash
            let mut salted_input = Vec::new();
            salted_input.extend_from_slice(&commit.salt);
            salted_input.extend_from_slice(&result);

            let salted_hash = <T as frame_system::Config>::Hashing::hash_of(&salted_input);
            log::info!("🧂 salted Hash: {:?}", salted_hash);
            log::info!("🔒 committed Hash: {:?}", commit.result_hash);
            ensure!(salted_hash.as_ref() == commit.result_hash.as_ref(), Error::<T>::SaltVerificationFailed);

            let bounded_result: BoundedVec<u8, ConstU32<2048>> =
                result.clone().try_into().map_err(|_| Error::<T>::InputDataTooLarge)?;

            let mut reveals = Reveals::<T>::get(job_id).unwrap_or_default();
            ensure!(!reveals.iter().any(|r| worker == r.worker), Error::<T>::AlreadyRevealed);

            let reveal = Reveal {
                worker: worker.clone(),
                salt: commit.salt,
                result: bounded_result,
                revealed_at: current_block,
            };

            reveals.try_push(reveal).map_err(|_| Error::<T>::AlreadyCommitted)?;
            Reveals::<T>::insert(job_id, reveals);
            Self::deposit_event(Event::ResultRevealed { job_id, worker });

            Ok(())
        }

        /// Finalize a job by determining consensus and distributing rewards
        #[pallet::call_index(5)]
        #[pallet::weight(Weight::from_parts(50_000, 0) + <T as frame_system::Config>::DbWeight::get().reads_writes(5, 5))]
        pub fn finalize_job(origin: OriginFor<T>, job_id: JobId) -> DispatchResult {
            let _caller = ensure_signed(origin)?;

            let mut job = Jobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;
            ensure!(job.status != JobStatus::Completed, Error::<T>::JobAlreadyFinalized);

            let current_block = frame_system::Pallet::<T>::block_number();
            ensure!(current_block > job.reveal_deadline, Error::<T>::InvalidJobPhase);

            let reveals = Reveals::<T>::get(job_id).ok_or(Error::<T>::InsufficientReveals)?;
            ensure!(!reveals.is_empty(), Error::<T>::InsufficientReveals);

            let consensus_result = Self::determine_consensus(&reveals)?;
            Results::<T>::insert(job_id, consensus_result.clone());

            Self::distribute_rewards_and_slash(job_id, &job, &reveals, &consensus_result)?;

            job.status = JobStatus::Completed;
            job.result = consensus_result.clone();
            Jobs::<T>::insert(job_id, job);

            Self::deposit_event(Event::JobFinalized {
                job_id,
                result: consensus_result.to_vec(),
            });

            Ok(())
        }

        #[pallet::call_index(6)]
        #[pallet::weight(Weight::from_parts(50_000, 0) + <T as frame_system::Config>::DbWeight::get().reads_writes(5, 5))]
        pub fn request_remote_job(
            origin: OriginFor<T>,
            dest_para_id: u32,
            input_data: Vec<u8>,
            bounty: u128,
            program_hash: <T as frame_system::Config>::Hash,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let origin_para_id = u32::from(T::ParaId::get());
            // Convert Vec<u8> to BoundedVec
            let bounded_input: BoundedVec<u8, T::MaxInputBytes> = 
                input_data.try_into().map_err(|_| Error::<T>::InputDataTooLarge)?;

            // Call your internal logic function
            Self::do_request_remote_job(
                sender, 
                dest_para_id, 
                bounded_input, 
                bounty, 
                program_hash,
                origin_para_id
            )
        }

        /// Receive a job result from a remote parachain (called via XCM)
        /// NOTE: You will need this for the demo to work
        #[pallet::call_index(7)] // Make sure this index is unique
        #[pallet::weight(Weight::from_parts(50_000, 0) + <T as frame_system::Config>::DbWeight::get().reads_writes(5, 5))]
        pub fn receive_remote_job_result(
            origin: OriginFor<T>, // This should be an XCM origin
            job_id: <T as frame_system::Config>::Hash,
            result_hash: <T as frame_system::Config>::Hash,
            success: bool,
        ) -> DispatchResult {

            ensure_root(origin.clone()).or_else(|_| ensure_signed(origin.clone()).map(|_| ()))?;
            
            Self::do_receive_remote_job_result(job_id, result_hash, success)
        }

        /// Handle XCM job submission from remote parachain
        #[pallet::call_index(8)]
        #[pallet::weight(Weight::from_parts(50_000, 0) + <T as frame_system::Config>::DbWeight::get().reads_writes(3, 3))]
        pub fn xcm_handle_job_submission(
            origin: OriginFor<T>,
            sender: T::AccountId,
            input_data: Vec<u8>,
            bounty: u128,
            job_id: <T as frame_system::Config>::Hash,
            program_hash: <T as frame_system::Config>::Hash,
            origin_para_id: u32,
        ) -> DispatchResult {
            let _ = origin;  // Accept any origin for XCM passthrough
            
            let bounded_input: BoundedVec<u8, T::MaxInputBytes> = 
                input_data.try_into().map_err(|_| Error::<T>::InputDataTooLarge)?;
            
            Self::handle_xcm_job_submission(sender, bounded_input, bounty, job_id, program_hash, origin_para_id)
        }
    }
}