//! # XCM Job Client Module
//!
//! This module allows parachains to send job submission requests to remote Agora parachains
//! via XCM and receive results back.

use crate::*;
use codec::Encode;
use sp_std::vec::Vec;
use staging_xcm::prelude::*;
use sp_runtime::SaturatedConversion;
use sp_std::vec;

// Helper functions
impl<T: Config> Pallet<T> {
	/// Generate a unique job ID
	pub fn generate_job_id(
		sender: &T::AccountId, 
		dest_para_id: u32
	) -> Result<<T as frame_system::Config>::Hash, DispatchError> {
		let nonce = JobNonce::<T>::get();
		let new_nonce = nonce.checked_add(1).ok_or(Error::<T>::Overflow)?;
		JobNonce::<T>::put(new_nonce);

		let mut data = Vec::new();
		data.extend_from_slice(&sender.encode());
		data.extend_from_slice(&dest_para_id.encode());
		data.extend_from_slice(&nonce.encode());

		Ok(<T as frame_system::Config>::Hashing::hash_of(&data))
	}

	/// Encode submit_job call for remote execution
	pub fn encode_submit_job_call(
		_sender: T::AccountId,
		input: BoundedVec<u8, T::MaxInputBytes>,
		bounty: u128,
		_program_hash: <T as frame_system::Config>::Hash,
	) -> Result<Vec<u8>, DispatchError> {
		// Pallet index for Agora pallet (check your runtime configuration)
		let pallet_index: u8 = 51;
		// Call index for submit_job in Agora pallet
		let call_index: u8 = 0;

		let mut encoded = Vec::new();
		encoded.push(pallet_index);
		encoded.push(call_index);
		encoded.extend_from_slice(&input.encode());
		encoded.extend_from_slice(&bounty.encode());

		Ok(encoded)
	}

	/// Build XCM message for job request
	pub fn build_job_request_xcm(
		sender: T::AccountId,
		bounty: u128,
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
				fallback_max_weight: Some(Weight::from_parts(1_000_000_000, 64 * 1024)),
				call: call.into(),
			},
		]);

		Ok(message)
	}

	/// Send XCM to destination parachain
	pub fn send_xcm_to_parachain(
		para_id: u32,
		job_id: <T as frame_system::Config>::Hash,
		message: Xcm<()>,
	) -> DispatchResult {
		let dest = Location::new(1, [Parachain(para_id)]);

		let (ticket, _) = T::XcmSender::validate(&mut Some(dest.clone()), &mut Some(message))
			.map_err(|_| Error::<T>::XcmSendFailed)?;

		T::XcmSender::deliver(ticket).map_err(|_| Error::<T>::XcmSendFailed)?;

		Self::deposit_event(Event::XcmMessageSent { 
			destination: para_id, 
			message_hash: job_id 
		});

		Ok(())
	}

	/// Request a remote job via XCM
	pub fn do_request_remote_job(
		sender: T::AccountId,
		dest_para_id: u32,
		input: BoundedVec<u8, T::MaxInputBytes>,
		bounty: u128,
		program_hash: <T as frame_system::Config>::Hash,
	) -> DispatchResult {
		// Ensure sender has enough balance
		ensure!(
			T::Currency::balance(&sender) >= bounty,
			Error::<T>::InsufficientBalance
		);

		// Reserve the bounty
		T::Currency::hold(&HoldReason::JobBounty.into(), &sender, bounty)
			.map_err(|_| Error::<T>::InsufficientBalance)?;

		// Generate unique job ID
		let job_id = Self::generate_job_id(&sender, dest_para_id)?;

		// Store pending job
		PendingJobs::<T>::insert(job_id, (sender.clone(), dest_para_id, bounty));

		// Encode the remote call (submit_job on destination)
		let call = Self::encode_submit_job_call(sender.clone(), input.clone(), bounty, program_hash)?;

		// Build XCM message
		let xcm_message = Self::build_job_request_xcm(sender.clone(), bounty, call)?;

		// Send XCM
		Self::send_xcm_to_parachain(dest_para_id, job_id, xcm_message)?;

		Self::deposit_event(Event::RemoteJobRequested { 
			job_id, 
			sender, 
			dest_para_id, 
			bounty 
		});

		Ok(())
	}

	/// Receive job result from remote Agora parachain
	pub fn do_receive_remote_job_result(
		job_id: <T as frame_system::Config>::Hash,
		result_hash: <T as frame_system::Config>::Hash,
		success: bool,
	) -> DispatchResult {
		// Get pending job info
		let (requester, _dest_para_id, bounty) =
			PendingJobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;

		if success {
			// Unreserve the bounty (it was already transferred via XCM)
			let _ = T::Currency::release(
				&HoldReason::JobBounty.into(),
				&requester,
				bounty,
				frame::traits::tokens::Precision::Exact,
			);

			Self::deposit_event(Event::RemoteJobCompleted { 
				job_id, 
				result_hash 
			});
		} else {
			// Job failed, return the reserved bounty
			let _ = T::Currency::release(
				&HoldReason::JobBounty.into(),
				&requester,
				bounty,
				frame::traits::tokens::Precision::Exact,
			);

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
	pub fn do_cancel_remote_job(
		sender: T::AccountId,
		job_id: <T as frame_system::Config>::Hash
	) -> DispatchResult {
		// Get pending job info
		let (requester, _dest_para_id, bounty) =
			PendingJobs::<T>::get(job_id).ok_or(Error::<T>::JobNotFound)?;

		// Ensure only requester can cancel
		ensure!(requester == sender, DispatchError::BadOrigin);

		// Unreserve the bounty
		let _ = T::Currency::release(
			&HoldReason::JobBounty.into(),
			&requester,
			bounty,
			frame::traits::tokens::Precision::Exact,
		);

		// Remove from pending jobs
		PendingJobs::<T>::remove(job_id);

		Ok(())
	}
}