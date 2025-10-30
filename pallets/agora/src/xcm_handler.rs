//! XCM Handler for Agora Pallet
//! 
//! This module handles incoming XCM messages that request job submissions
//! and sends results back to origin parachains.

use crate::*;
use codec::{Decode, Encode};
use sp_std::vec::Vec;
use staging_xcm::prelude::*;
use sp_std::vec;
use staging_xcm::prelude::Weight;

impl<T: Config> Pallet<T> {
	/// Handle incoming XCM transact for job submission

	/// This is called when a remote parachain sends a job request via XCM
	pub fn handle_xcm_job_submission(
		sender: T::AccountId,
		input: BoundedVec<u8, T::MaxInputBytes>,
		bounty: u128,
		job_id: <T as frame_system::Config>::Hash,
		program_hash: <T as frame_system::Config>::Hash,
		origin_para_id: u32,
	) -> DispatchResult {
		let local_job_id = Self::do_submit_job(sender.clone(), input, bounty, origin_para_id)?;
		
		RemoteJobInfo::<T>::insert(local_job_id, (origin_para_id, job_id));
		
		Self::deposit_event(Event::XcmJobSubmitted {
			job_id: local_job_id,
			creator: sender,
			bounty,
			origin_para_id,
		});
		
		Ok(())
	}
	
	/// Send job result back to origin parachain
	/// Called when a remote job completes
	pub fn send_job_result_to_origin(
		local_job_id: JobId,
		result_hash: <T as frame_system::Config>::Hash,
		success: bool,
	) -> DispatchResult {
		// Get the origin parachain ID
		let (origin_para_id, remote_job_id) = RemoteJobInfo::<T>::get(local_job_id)
        	.ok_or(Error::<T>::JobNotFound)?;
		
		// Encode result notification call
		let notification = Self::encode_job_result_notification(remote_job_id, result_hash, success)?;
		
		// Build XCM message
		let message = Xcm(vec![
			UnpaidExecution {
				weight_limit: Unlimited,
				check_origin: None,
			},
			Transact {
				origin_kind: OriginKind::SovereignAccount,
				fallback_max_weight: Some(Weight::from_parts(500_000_000, 32 * 1024)),
				call: notification.into(),
			},
		]);
		
		// Send XCM to origin parachain
		let dest = Location::new(1, [Parachain(origin_para_id)]);
		
		let (ticket, _) = T::XcmSender::validate(&mut Some(dest), &mut Some(message))
			.map_err(|_| Error::<T>::XcmSendFailed)?;
		
		T::XcmSender::deliver(ticket)
			.map_err(|_| Error::<T>::XcmSendFailed)?;
		
		// Clean up storage
		RemoteJobInfo::<T>::remove(local_job_id);
		
		Self::deposit_event(Event::JobResultQueried {
			job_id: local_job_id,
			result: vec![],
			origin_para_id,
		});
		
		Ok(())
	}
	
	/// Encode the job result notification call for the client parachain
	/// This creates a call to `receive_remote_job_result` on the Agora pallet
	fn encode_job_result_notification(
		job_id: <T as frame_system::Config>::Hash,
		result_hash: <T as frame_system::Config>::Hash,
		success: bool,
	) -> Result<Vec<u8>, DispatchError> {
		// Pallet index for Agora pallet on client parachain
		// This should match the pallet index in the client's runtime
		let pallet_index: u8 = 51;
		// Call index for receive_remote_job_result
		let call_index: u8 = 9;
		
		let mut encoded = Vec::new();
		encoded.push(pallet_index);
		encoded.push(call_index);
		encoded.extend_from_slice(&job_id.encode());
		encoded.extend_from_slice(&result_hash.encode());
		encoded.extend_from_slice(&success.encode());
		
		Ok(encoded)
	}
	
	/// Internal function to submit a job
	/// This allows both local and XCM calls to use the same logic
	fn do_submit_job(
		sender: T::AccountId,
		input: BoundedVec<u8, T::MaxInputBytes>,
		bounty: u128,
		origin_para_id: u32,
	) -> Result<JobId, DispatchError> {
		// Ensure bounty meets minimum
		ensure!(bounty >= T::MinJobBounty::get(), Error::<T>::InsufficientBounty);
		
		// Check balance
		let balance = T::Currency::balance(&sender);
		ensure!(balance >= bounty, Error::<T>::InsufficientBalance);
		
		// Hold the bounty
		T::Currency::hold(&HoldReason::JobBounty.into(), &sender, bounty)?;
		
		// Generate job ID
		let job_id = NextJobId::<T>::get();
		NextJobId::<T>::put(job_id.saturating_add(1));
		
		// Get current block number
		let current_block = frame_system::Pallet::<T>::block_number();
		let commit_deadline = current_block + T::CommitPhaseDuration::get();
		let reveal_deadline = commit_deadline + T::RevealPhaseDuration::get();
		
		// Convert to proper bounded vec size
		let bounded_input: BoundedVec<u8, ConstU32<1024>> = input.to_vec()
			.try_into()
			.map_err(|_| Error::<T>::InputDataTooLarge)?;
		
		// Create job
		let job = Job {
			creator: sender.clone(),
			bounty,
			job_type: JobType::Computation,
			input_data: bounded_input,
			status: JobStatus::Pending,
			created_at: current_block,
			commit_deadline,
			reveal_deadline,
			origin_para_id,
			result: BoundedVec::default(),
		};
		
		// Store job
		Jobs::<T>::insert(job_id, job);
		
		Ok(job_id)
	}
	
	/// Check if a job originated from XCM and send results if so
	/// This should be called after a job completes successfully
	pub fn maybe_send_remote_result(
		job_id: JobId, // This is the local numeric ID
		result_hash: <T as frame_system::Config>::Hash
	) -> DispatchResult {
		if RemoteJobInfo::<T>::contains_key(job_id) { // <-- Use RemoteJobInfo
			Self::send_job_result_to_origin(job_id, result_hash, true)?;
		}
		Ok(())
	}
	
	/// Check if a job originated from XCM and send failure notification
	/// This should be called if a job fails
	pub fn maybe_send_remote_failure(job_id: JobId) -> DispatchResult {
		if RemoteJobInfo::<T>::contains_key(job_id) { // <-- Use RemoteJobInfo
			let zero_hash = <T as frame_system::Config>::Hash::default();
			Self::send_job_result_to_origin(job_id, zero_hash, false)?;
		}
		Ok(())
	}
}