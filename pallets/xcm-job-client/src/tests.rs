use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use sp_runtime::traits::Hash;

#[test]
fn request_remote_job_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let sender = 1;
		let dest_para_id = 2000;
		let input: BoundedVec<u8, ConstU32<1024>> = 
			BoundedVec::try_from(vec![1, 2, 3, 4]).unwrap();
		let bounty = 1000;
		let program_hash = <Test as frame_system::Config>::Hashing::hash(b"program");

		// Request remote job
		assert_ok!(XcmJobClient::request_remote_job(
			RuntimeOrigin::signed(sender),
			dest_para_id,
			input.clone(),
			bounty,
			program_hash,
		));

		// Check that balance was reserved
		assert_eq!(Balances::reserved_balance(sender), bounty);

		// Check job nonce increased
		assert_eq!(XcmJobClient::job_nonce(), 1);

		// Check event was emitted
		System::assert_has_event(
			Event::RemoteJobRequested {
				job_id: XcmJobClient::generate_job_id(&sender, dest_para_id).unwrap(),
				sender,
				dest_para_id,
				bounty,
			}
			.into(),
		);
	});
}

#[test]
fn request_remote_job_fails_insufficient_balance() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let sender = 1;
		let dest_para_id = 2000;
		let input: BoundedVec<u8, ConstU32<1024>> = 
			BoundedVec::try_from(vec![1, 2, 3, 4]).unwrap();
		let bounty = 10000000; // More than sender has
		let program_hash = <Test as frame_system::Config>::Hashing::hash(b"program");

		// Request should fail
		assert_noop!(
			XcmJobClient::request_remote_job(
				RuntimeOrigin::signed(sender),
				dest_para_id,
				input,
				bounty,
				program_hash,
			),
			Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
fn receive_remote_job_result_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let sender = 1;
		let dest_para_id = 2000;
		let input: BoundedVec<u8, ConstU32<1024>> = 
			BoundedVec::try_from(vec![1, 2, 3, 4]).unwrap();
		let bounty = 1000;
		let program_hash = <Test as frame_system::Config>::Hashing::hash(b"program");

		// Request remote job first
		assert_ok!(XcmJobClient::request_remote_job(
			RuntimeOrigin::signed(sender),
			dest_para_id,
			input,
			bounty,
			program_hash,
		));

		let job_id = XcmJobClient::generate_job_id(&sender, dest_para_id).unwrap();
		let result_hash = <Test as frame_system::Config>::Hashing::hash(b"result");

		// Simulate receiving result
		assert_ok!(XcmJobClient::receive_remote_job_result(
			RuntimeOrigin::signed(sender),
			job_id,
			result_hash,
			true,
		));

		// Check event
		System::assert_has_event(
			Event::RemoteJobCompleted { job_id, result_hash }.into(),
		);

		// Check pending job was removed
		assert_eq!(XcmJobClient::pending_jobs(job_id), None);
	});
}

#[test]
fn cancel_remote_job_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let sender = 1;
		let dest_para_id = 2000;
		let input: BoundedVec<u8, ConstU32<1024>> = 
			BoundedVec::try_from(vec![1, 2, 3, 4]).unwrap();
		let bounty = 1000;
		let program_hash = <Test as frame_system::Config>::Hashing::hash(b"program");

		// Request remote job
		assert_ok!(XcmJobClient::request_remote_job(
			RuntimeOrigin::signed(sender),
			dest_para_id,
			input,
			bounty,
			program_hash,
		));

		let job_id = XcmJobClient::generate_job_id(&sender, dest_para_id).unwrap();
		let reserved_before = Balances::reserved_balance(sender);

		// Cancel job
		assert_ok!(XcmJobClient::cancel_remote_job(
			RuntimeOrigin::signed(sender),
			job_id,
		));

		// Check balance was unreserved
		assert_eq!(Balances::reserved_balance(sender), 0);

		// Check pending job was removed
		assert_eq!(XcmJobClient::pending_jobs(job_id), None);
	});
}