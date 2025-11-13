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
use staging_xcm::prelude::Weight;

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

	/// Get the pallet's holding account for bounties
    pub fn pallet_account() -> T::AccountId {
        T::PalletId::get().into_account_truncating()
    }

	/// Encode submit_job call for remote execution
	pub fn encode_submit_job_call(
		sender: T::AccountId,
		input: BoundedVec<u8, T::MaxInputBytes>,
		bounty: u128,
		reputation: u32,
		job_id: <T as frame_system::Config>::Hash,
		origin_para_id: u32,
	) -> Result<Vec<u8>, DispatchError> {
		let input_data: Vec<u8> = input.to_vec();
		let sender_encoded = sender.encode();
		
		let mut encoded = Vec::new();
		encoded.push(51);
		encoded.push(8);
		encoded.extend_from_slice(&sender_encoded);
		encoded.extend_from_slice(&input_data.encode());
		encoded.extend_from_slice(&bounty.encode());
		encoded.extend_from_slice(&job_id.encode());
		encoded.extend_from_slice(&reputation.encode());
		encoded.extend_from_slice(&origin_para_id.encode());
		
		Ok(encoded)
	}

	/// Build XCM message for job request
	pub fn build_job_request_xcm(
		sender: T::AccountId,
		bounty: u128,
		call: Vec<u8>,
		dest_para_id: u32,
		
	) -> Result<Xcm<()>, DispatchError> {
		// Asset in local context (Here = this parachain's native token)
		let asset: Asset = (Here, bounty).into();

		// Beneficiary on destination (the original sender)
		let mut sender_bytes = [0u8; 32];
		let encoded = sender.encode();
		let len = encoded.len().min(32);
		sender_bytes[..len].copy_from_slice(&encoded[..len]);
		
		let beneficiary = Location::new(
			0,
			[AccountId32 { network: None, id: sender_bytes }]
		);

		let message = Xcm(vec![
			ReserveAssetDeposited(vec![asset.clone()].into()),
			BuyExecution { 
				fees: asset, 
				weight_limit: Unlimited 
			},
			Transact {
				origin_kind: OriginKind::SovereignAccount,
				fallback_max_weight: Some(Weight::from_parts(1_000_000_000, 64 * 1024)),
				call: call.into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: Wild(All),
				beneficiary,
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
		reputation: u32,
		origin_para_id: u32,
	) -> DispatchResult {
		// Ensure sender has enough balance
		ensure!(
			T::Currency::balance(&sender) >= bounty,
			Error::<T>::InsufficientBalance
		);

		T::Currency::hold(&HoldReason::JobBounty.into(), &sender, bounty)?;

		// Generate unique job ID for tracking
		let job_id = Self::generate_job_id(&sender, dest_para_id)?;

		// Store pending job locally with sender info for refund if needed
		PendingJobs::<T>::insert(job_id, (sender.clone(), dest_para_id, bounty));

		// Encode the remote call (submit_job on destination)
		let call = Self::encode_submit_job_call(
			sender.clone(), 
			input.clone(), 
			bounty, 
			reputation, 
			job_id,
			origin_para_id
		)?;

		let xcm_message = Self::build_job_request_xcm(
			sender.clone(), 
			bounty, 
			call,
			dest_para_id
		)?;

		// Send XCM to destination parachain
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
		result: Vec<u8>,
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
				result 
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