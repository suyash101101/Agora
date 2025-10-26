//! XCM Handler for Agora Pallet
//! 
//! This module handles incoming XCM messages that request job submissions
//! and sends results back to origin parachains.

use super::*;
use codec::{Decode, Encode};
use frame_support::traits::Get;
use sp_runtime::traits::Hash;
use sp_std::vec::Vec;
use staging_xcm::prelude::*;

impl<T: Config> Pallet<T> {
	/// Handle incoming XCM transact for job submission
	/// This is called when a remote parachain sends a job request via XCM
	pub fn handle_xcm_job_submission(
		sender: T::AccountId,
		encoded_params: Vec<u8>,
		origin_para_id: u32,
	) -> DispatchResult {
		// Decode the call parameters
		// Format: [input, bounty, program_hash]
		let mut cursor = &encoded_params[..];
		
		let input = BoundedVec::<u8, T::MaxInputBytes>::decode(&mut cursor)
			.map_err(|_| Error::<T>::InvalidCallData)?;
		let bounty = BalanceOf::<T>::decode(&mut cursor)
			.map_err(|_| Error::<T>::InvalidCallData)?;
		let program_hash = T::Hash::decode(&mut cursor)
			.map_err(|_| Error::<T>::InvalidCallData)?;
		
		// Create the job using the existing submit_job logic
		let job_id = Self::do_submit_job(sender.clone(), input, bounty, program_hash)?;
		
		// Store the origin parachain ID so we can send results back
		RemoteJobOrigins::<T>::insert(job_id, origin_para_id);
		
		Self::deposit_event(Event::RemoteJobSubmitted {
			job_id,
			origin_para_id,
			sender,
		});
		
		Ok(())
	}
	
	/// Send job result back to origin parachain
	/// Called when a remote job completes
	pub fn send_job_result_to_origin(
		job_id: T::Hash,
		result_hash: T::Hash,
		success: bool,
	) -> DispatchResult {
		// Get the origin parachain ID
		let origin_para_id = RemoteJobOrigins::<T>::get(job_id)
			.ok_or(Error::<T>::NotRemoteJob)?;
		
		// Encode result notification call
		let notification = Self::encode_job_result_notification(job_id, result_hash, success)?;
		
		// Build XCM message
		let message = Xcm(vec![
			UnpaidExecution {
				weight_limit: Unlimited,
				check_origin: None,
			},
			Transact {
				origin_kind: OriginKind::SovereignAccount,
				fallback_max_weight: Weight::from_parts(500_000_000, 32 * 1024),
				call: notification.into(),
			},
		]);
		
		// Send XCM to origin parachain
		let dest = Location::new(1, [Parachain(origin_para_id)]);
		
		T::XcmSender::send_xcm(dest, message)
			.map_err(|_| Error::<T>::XcmSendFailed)?;
		
		// Clean up storage
		RemoteJobOrigins::<T>::remove(job_id);
		
		Self::deposit_event(Event::RemoteJobResultSent {
			job_id,
			dest_para_id: origin_para_id,
		});
		
		Ok(())
	}
	
	/// Encode the job result notification call for the client parachain
	/// This creates a call to `receive_remote_job_result` on the XcmJobClient pallet
	fn encode_job_result_notification(
		job_id: T::Hash,
		result_hash: T::Hash,
		success: bool,
	) -> Result<Vec<u8>, DispatchError> {
		// Pallet index for XcmJobClient (you'll need to configure this)
		// Default to 52 (assuming Agora is 51)
		let pallet_index: u8 = 52;
		// Call index for receive_remote_job_result
		let call_index: u8 = 1;
		
		let mut encoded = Vec::new();
		encoded.push(pallet_index);
		encoded.push(call_index);
		encoded.extend_from_slice(&job_id.encode());
		encoded.extend_from_slice(&result_hash.encode());
		encoded.extend_from_slice(&success.encode());
		
		Ok(encoded)
	}
	
	/// Internal function to submit a job (extracted from submit_job extrinsic)
	/// This allows both local and XCM calls to use the same logic
	fn do_submit_job(
		sender: T::AccountId,
		input: BoundedVec<u8, T::MaxInputBytes>,
		bounty: BalanceOf<T>,
		program_hash: T::Hash,
	) -> Result<T::Hash, DispatchError> {
		// Ensure bounty meets minimum
		ensure!(bounty >= T::MinJobBounty::get(), Error::<T>::BountyTooLow);
		
		// Generate unique job ID
		let job_id = T::Hashing::hash_of(&(&sender, &input, &program_hash, &frame_system::Pallet::<T>::block_number()));
		
		// Ensure job doesn't already exist
		ensure!(!Jobs::<T>::contains_key(job_id), Error::<T>::JobAlreadyExists);
		
		// Hold the bounty
		T::Currency::hold(&HoldReason::JobBounty.into(), &sender, bounty)?;
		
		// Create job
		let job = Job {
			creator: sender.clone(),
			input: input.clone(),
			bounty,
			program_hash,
			status: JobStatus::Open,
			commit_deadline: frame_system::Pallet::<T>::block_number() + T::CommitPhaseDuration::get(),
			reveal_deadline: frame_system::Pallet::<T>::block_number() 
				+ T::CommitPhaseDuration::get() 
				+ T::RevealPhaseDuration::get(),
		};
		
		// Store job
		Jobs::<T>::insert(job_id, job);
		
		// Emit event
		Self::deposit_event(Event::JobSubmitted {
			job_id,
			creator: sender,
			bounty,
		});
		
		Ok(job_id)
	}
	
	/// Check if a job originated from XCM and send results if so
	/// This should be called after a job completes successfully
	pub fn maybe_send_remote_result(job_id: T::Hash, result_hash: T::Hash) -> DispatchResult {
		if RemoteJobOrigins::<T>::contains_key(job_id) {
			Self::send_job_result_to_origin(job_id, result_hash, true)?;
		}
		Ok(())
	}
	
	/// Check if a job originated from XCM and send failure notification
	/// This should be called if a job fails
	pub fn maybe_send_remote_failure(job_id: T::Hash) -> DispatchResult {
		if RemoteJobOrigins::<T>::contains_key(job_id) {
			// Use zero hash to indicate failure
			Self::send_job_result_to_origin(job_id, T::Hash::default(), false)?;
		}
		Ok(())
	}
}