#![cfg_attr(not(feature = "std"), no_std)]

//! # XCM Job Client Pallet
//!
//! This pallet allows parachains to send job submission requests to remote Agora parachains
//! via XCM and receive results back.

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use codec::{Decode, Encode};
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, Imbalance, ReservableCurrency},
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Hash;
	use sp_std::vec::Vec;
	use staging_xcm::prelude::*;
	use sp_runtime::SaturatedConversion;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Currency type for handling payments
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// XCM sender
		type XcmSender: SendXcm;

		/// Maximum size of job input data
		#[pallet::constant]
		type MaxInputBytes: Get<u32>;

		/// Weight information for extrinsics
		type WeightInfo: WeightInfo;

		/// Origin that can send XCM messages (usually the sovereign account)
		type XcmExecuteOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	/// Pending remote jobs waiting for results
	#[pallet::storage]
	#[pallet::getter(fn pending_jobs)]
	pub type PendingJobs<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::Hash, // job_id
		(T::AccountId, u32, BalanceOf<T>), // (requester, dest_para_id, bounty)
		OptionQuery,
	>;

	/// Nonce for generating unique job IDs
	#[pallet::storage]
	#[pallet::getter(fn job_nonce)]
	pub type JobNonce<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Remote job request sent
		RemoteJobRequested {
			job_id: T::Hash,
			sender: T::AccountId,
			dest_para_id: u32,
			bounty: BalanceOf<T>,
		},
		/// Remote job completed and result received
		RemoteJobCompleted {
			job_id: T::Hash,
			result_hash: T::Hash,
		},
		/// Remote job failed
		RemoteJobFailed {
			job_id: T::Hash,
			reason: Vec<u8>,
		},
		/// XCM message sent successfully
		XcmMessageSent {
			destination: u32,
			message_hash: T::Hash,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// XCM send failed
		XcmSendFailed,
		/// Insufficient balance for remote job
		InsufficientBalance,
		/// Job not found
		JobNotFound,
		/// Invalid input data
		InvalidInput,
		/// Input data too large
		InputTooLarge,
		/// Already has pending job
		PendingJobExists,
		/// Arithmetic overflow
		Overflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Send a job submission request to remote Agora parachain via XCM
		///
		/// # Parameters
		/// - `origin`: The account requesting the job
		/// - `dest_para_id`: The destination parachain ID running Agora
		/// - `input`: The job input data
		/// - `bounty`: The payment for the job
		/// - `program_hash`: The hash of the program to execute
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::request_remote_job())]
		pub fn request_remote_job(
			origin: OriginFor<T>,
			dest_para_id: u32,
			input: BoundedVec<u8, T::MaxInputBytes>,
			bounty: BalanceOf<T>,
			program_hash: T::Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Ensure sender has enough balance
			ensure!(
				T::Currency::free_balance(&sender) >= bounty,
				Error::<T>::InsufficientBalance
			);

			// Reserve the bounty
			T::Currency::reserve(&sender, bounty)
				.map_err(|_| Error::<T>::InsufficientBalance)?;

			// Generate unique job ID
			let job_id = Self::generate_job_id(&sender, dest_para_id)?;

			// Store pending job
			PendingJobs::<T>::insert(job_id, (sender.clone(), dest_para_id, bounty));

			// Encode the remote call (submit_job on destination)
			let call = Self::encode_submit_job_call(
				sender.clone(),
				input.clone(),
				bounty,
				program_hash,
			)?;

			// Build XCM message
			let xcm_message = Self::build_job_request_xcm(sender.clone(), bounty, call)?;

			// Send XCM
			Self::send_xcm_to_parachain(dest_para_id, job_id, xcm_message)?;

			Self::deposit_event(Event::RemoteJobRequested {
				job_id,
				sender,
				dest_para_id,
				bounty,
			});

			Ok(())
		}

		/// Receive job result from remote Agora parachain
		/// This should only be called by XCM execution
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::receive_remote_job_result())]
		pub fn receive_remote_job_result(
			origin: OriginFor<T>,
			job_id: T::Hash,
			result_hash: T::Hash,
			success: bool,
		) -> DispatchResult {
			// Ensure this comes from XCM (sovereign account of sibling parachain)
			let _ = ensure_signed(origin)?;

			// Get pending job info
			let (requester, _dest_para_id, bounty) =
				PendingJobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;

			if success {
				// Unreserve the bounty (it was already transferred via XCM)
				let _ = T::Currency::unreserve(&requester, bounty);

				Self::deposit_event(Event::RemoteJobCompleted { job_id, result_hash });
			} else {
				// Job failed, return the reserved bounty
				let _ = T::Currency::unreserve(&requester, bounty);

				Self::deposit_event(Event::RemoteJobFailed {
					job_id,
					reason: b"Job execution failed".to_vec(),
				});
			}

			// Remove from pending jobs
			PendingJobs::<T>::remove(job_id);

			Ok(())
		}

		/// Cancel a pending remote job request
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::cancel_remote_job())]
		pub fn cancel_remote_job(origin: OriginFor<T>, job_id: T::Hash) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Get pending job info
			let (requester, _dest_para_id, bounty) =
				PendingJobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;

			// Ensure only requester can cancel
			ensure!(requester == sender, DispatchError::BadOrigin);

			// Unreserve the bounty
			let _ = T::Currency::unreserve(&requester, bounty);

			// Remove from pending jobs
			PendingJobs::<T>::remove(job_id);

			Ok(())
		}
	}

	// Helper functions
	impl<T: Config> Pallet<T> {
		/// Generate a unique job ID
	fn generate_job_id(sender: &T::AccountId, dest_para_id: u32) -> Result<T::Hash, DispatchError>
	where
		T: frame_system::Config,
		T::Hashing: sp_runtime::traits::Hash<Output = T::Hash>,
	{
		let nonce = JobNonce::<T>::get();
		let new_nonce = nonce.checked_add(1).ok_or(Error::<T>::Overflow)?;
		JobNonce::<T>::put(new_nonce);

		let mut data = Vec::new();
		data.extend_from_slice(&sender.encode());
		data.extend_from_slice(&dest_para_id.encode());
		data.extend_from_slice(&nonce.encode());

		Ok(T::Hashing::hash_of(&data))
	}


		/// Encode submit_job call for remote execution
		fn encode_submit_job_call(
			sender: T::AccountId,
			input: BoundedVec<u8, T::MaxInputBytes>,
			bounty: BalanceOf<T>,
			program_hash: T::Hash,
		) -> Result<Vec<u8>, DispatchError> {
			// Pallet index for Agora pallet (51 from your runtime)
			// You may need to adjust this based on your runtime configuration
			let pallet_index: u8 = 51;
			// Call index for submit_job in Agora pallet
			let call_index: u8 = 0;

			let mut encoded = Vec::new();
			encoded.push(pallet_index);
			encoded.push(call_index);
			encoded.extend_from_slice(&input.encode());
			encoded.extend_from_slice(&bounty.encode());
			encoded.extend_from_slice(&program_hash.encode());

			Ok(encoded)
		}

		/// Build XCM message for job request
		fn build_job_request_xcm(
			sender: T::AccountId,
			bounty: BalanceOf<T>,
			call: Vec<u8>,
		) -> Result<Xcm<()>, DispatchError> {
			let asset: Asset = (Here, bounty.saturated_into::<u128>()).into();

			let beneficiary = AccountId32 {
				network: None,
				id: sender.encode().try_into().unwrap_or([0u8; 32]),
			};

			let message = Xcm(vec![
				// Withdraw the bounty from sender's sovereign account
				WithdrawAsset(asset.clone().into()),
				// Buy execution on destination
				BuyExecution { fees: asset.clone(), weight_limit: Unlimited },
				// Deposit asset to beneficiary (job creator on remote chain)
				DepositAsset {
					assets: All.into(),
					beneficiary: beneficiary.into(),
				},
				// Execute the submit_job call
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					fallback_max_weight: Some(sp_runtime::Weight::from_parts(1_000_000_000, 64 * 1024)),
					call: call.into(),
				},
			]);

			Ok(message)
		}

		/// Send XCM to destination parachain
		fn send_xcm_to_parachain(
			para_id: u32,
			job_id: T::Hash,
			message: Xcm<()>,
		) -> DispatchResult {
			let dest = Location::new(1, [Parachain(para_id)]);

			let (ticket, _) = T::XcmSender::validate(&mut Some(dest.clone()), &mut Some(message))
				.map_err(|_| Error::<T>::XcmSendFailed)?;

			T::XcmSender::deliver(ticket).map_err(|_| Error::<T>::XcmSendFailed)?;

			Self::deposit_event(Event::XcmMessageSent {
				destination: para_id,
				message_hash: job_id,
			});

			Ok(())
		}
	}
}