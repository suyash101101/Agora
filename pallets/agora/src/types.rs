use codec::{Decode, Encode, MaxEncodedLen};
use frame::prelude::*;
use scale_info::TypeInfo;

/// Unique identifier for a job
pub type JobId = u64;

/// Job type enumeration
#[derive(
	Encode,
	Decode,
	Clone,
	Copy,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum JobType {
	/// API request job
	ApiRequest,
	/// Computation job
	Computation,
}

/// Job status enumeration
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum JobStatus {
	/// Job is pending execution
	Pending,
	/// Job is in commit phase
	CommitPhase,
	/// Job is in reveal phase
	RevealPhase,
	/// Job has been completed
	Completed,
	/// Job has failed
	Failed,
}

/// Job structure containing all job information
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Job<T: frame_system::Config> {
	/// Account that created the job
	pub creator: T::AccountId,
	/// Bounty amount locked for this job
	pub bounty: u128,
	/// Type of job
	pub job_type: JobType,
	/// Input data for the job (limited size)
	pub input_data: BoundedVec<u8, ConstU32<1024>>,
	/// Current status of the job
	pub status: JobStatus,
	/// Block number when job was created
	pub created_at: BlockNumberFor<T>,
	/// Deadline for commit phase
	pub commit_deadline: BlockNumberFor<T>,
	/// Deadline for reveal phase
	pub reveal_deadline: BlockNumberFor<T>,
	/// Optional parachain ID if job submitted via XCM (0 = local chain)
	pub origin_para_id: u32,
	/// Actual result bytes (populated after reveal)
	pub result: BoundedVec<u8, ConstU32<2048>>,
	/// Minimum Reputation required for workers to participate
	pub reputation: u32,
}

/// Worker information structure
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct WorkerInfo<T: frame_system::Config> {
	/// Amount staked by the worker
	pub stake: u128,
	/// Reputation score (0-1000)
	pub reputation: u32,
	/// Whether the worker is currently active
	pub is_active: bool,
	/// Block number when worker registered
	pub registered_at: BlockNumberFor<T>,
}

/// Commit structure for storing hashed results
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Commit<T: frame_system::Config> {
	/// Worker who made the commit
	pub worker: T::AccountId,
	/// Salt used in the commit (32 bytes)
	pub salt: [u8; 32],
	/// Hash of salt + result (prevents preimage grinding)
	pub result_hash: T::Hash,
	/// Block number when commit was made
	pub committed_at: BlockNumberFor<T>,
}

/// Reveal structure for storing actual results
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct Reveal<T: frame_system::Config> {
	/// Worker who revealed
	pub worker: T::AccountId,
	/// Salt used in the commit (32 bytes)
	pub salt: [u8; 32],
	/// Actual result data
	pub result: BoundedVec<u8, ConstU32<2048>>,
	/// Block number when revealed
	pub revealed_at: BlockNumberFor<T>,
}

/// OCW Job Status for tracking off-chain worker execution
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum OCWJobStatus {
	/// Job is pending OCW execution
	PendingExecution,
	/// Job is being executed by OCW
	Executing,
	/// Job execution completed, waiting for commit
	ExecutionCompleted,
	/// Job committed, waiting for reveal
	Committed,
	/// Job revealed, waiting for finalization
	Revealed,
	/// Job completed by OCW
	Completed,
	/// Job failed during OCW execution
	Failed,
}

/// OCW Execution State for tracking job execution
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
pub struct OCWExecutionState<T: frame_system::Config> {
	/// Job ID
	pub job_id: JobId,
	/// Execution status
	pub status: OCWJobStatus,
	/// Computed result
	pub result: BoundedVec<u8, ConstU32<2048>>,
	/// Salt used for commit
	pub salt: [u8; 32],
	/// Commit hash
	pub commit_hash: [u8; 32],
	/// Block when execution started
	pub execution_start_block: BlockNumberFor<T>,
	/// Block when commit was submitted
	pub commit_block: Option<BlockNumberFor<T>>,
	/// Block when reveal was submitted
	pub reveal_block: Option<BlockNumberFor<T>>,
}

