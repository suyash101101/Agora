use crate::{mock::*, Error, Event};
use frame::prelude::*;
use frame::testing_prelude::*;

// Type aliases for convenience - Test is imported from mock::*
type Agora = crate::Pallet<Test>;

#[test]
fn worker_registration_works() {
	new_test_ext().execute_with(|| {
		// Alice registers as a worker with 100 stake
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(1), 100));

		// Check worker is registered
		let worker_info = Agora::workers(1).unwrap();
		assert_eq!(worker_info.stake, 100);
		assert_eq!(worker_info.reputation, 500);
		assert!(worker_info.is_active);

		// Check event was emitted
		frame_system::Pallet::<Test>::assert_last_event(Event::WorkerRegistered { worker: 1, stake: 100 }.into());
	});
}

#[test]
fn worker_registration_fails_with_insufficient_stake() {
	new_test_ext().execute_with(|| {
		// Try to register with stake below minimum
		assert_noop!(
			Agora::register_worker(RuntimeOrigin::signed(1), 50),
			Error::<Test>::InsufficientStake
		);
	});
}

#[test]
fn duplicate_worker_registration_fails() {
	new_test_ext().execute_with(|| {
		// Register Alice
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(1), 100));

		// Try to register again
		assert_noop!(
			Agora::register_worker(RuntimeOrigin::signed(1), 100),
			Error::<Test>::WorkerAlreadyRegistered
		);
	});
}

#[test]
fn worker_unregistration_works() {
	new_test_ext().execute_with(|| {
		// Register and then unregister
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(1), 100));
		assert_ok!(Agora::unregister_worker(RuntimeOrigin::signed(1)));

		// Check worker is removed
		assert!(Agora::workers(1).is_none());

		// Check event was emitted
		frame_system::Pallet::<Test>::assert_last_event(Event::WorkerUnregistered { worker: 1 }.into());
	});
}

#[test]
fn job_submission_works() {
	new_test_ext().execute_with(|| {
		let input_data = vec![1, 2, 3, 4];
		let bounty = 100u128;

		// Submit job (1 = Computation)
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			input_data.clone(),
			bounty
		));

		// Check job was created
		let job = Agora::jobs(0).unwrap();
		assert_eq!(job.creator, 1);
		assert_eq!(job.bounty, bounty);
		assert_eq!(job.input_data.to_vec(), input_data);
		assert_eq!(job.status, crate::types::JobStatus::Pending);

		// Check event was emitted
		frame_system::Pallet::<Test>::assert_last_event(
			Event::JobSubmitted { job_id: 0, creator: 1, bounty: 100 }.into(),
		);
	});
}

#[test]
fn job_submission_fails_with_insufficient_bounty() {
	new_test_ext().execute_with(|| {
		let input_data = vec![1, 2, 3, 4];

		// Try to submit with bounty below minimum (1 = Computation)
		assert_noop!(
			Agora::submit_job(RuntimeOrigin::signed(1), 1, input_data, 25),
			Error::<Test>::InsufficientBounty
		);
	});
}

#[test]
fn commit_result_works() {
	new_test_ext().execute_with(|| {
		// Setup: Register worker and submit job (1 = Computation)
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 100));
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			vec![1, 2, 3],
			100
		));

		// Worker commits result
		let result = vec![42u8];
		let result_hash = frame::hashing::BlakeTwo256::hash(&result);

		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, result_hash));

		// Check commit was stored
		let commits = Agora::commits(0).unwrap();
		assert_eq!(commits.len(), 1);
		assert_eq!(commits[0].worker, 2);
		assert_eq!(commits[0].result_hash, result_hash);

		// Check event was emitted
		frame_system::Pallet::<Test>::assert_last_event(Event::ResultCommitted { job_id: 0, worker: 2 }.into());
	});
}

#[test]
fn commit_result_fails_for_unregistered_worker() {
	new_test_ext().execute_with(|| {
		// Submit job (1 = Computation)
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			vec![1, 2, 3],
			100
		));

		// Unregistered worker tries to commit
		let result_hash = frame::hashing::BlakeTwo256::hash(&[42u8]);
		assert_noop!(
			Agora::commit_result(RuntimeOrigin::signed(2), 0, result_hash),
			Error::<Test>::WorkerNotRegistered
		);
	});
}

#[test]
fn reveal_result_works() {
	new_test_ext().execute_with(|| {
		// Setup: Register worker, submit job, commit result (1 = Computation)
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 100));
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			vec![1, 2, 3],
			100
		));

		let result = vec![42u8];
		let result_hash = frame::hashing::BlakeTwo256::hash(&result);
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, result_hash));

		// Move past commit deadline
		frame_system::Pallet::<Test>::set_block_number(12);

		// Reveal result
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(2), 0, result.clone()));

		// Check reveal was stored
		let reveals = Agora::reveals(0).unwrap();
		assert_eq!(reveals.len(), 1);
		assert_eq!(reveals[0].worker, 2);
		assert_eq!(reveals[0].result.to_vec(), result);

		// Check event was emitted
		frame_system::Pallet::<Test>::assert_last_event(Event::ResultRevealed { job_id: 0, worker: 2 }.into());
	});
}

#[test]
fn reveal_result_fails_with_wrong_hash() {
	new_test_ext().execute_with(|| {
		// Setup (1 = Computation)
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 100));
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			vec![1, 2, 3],
			100
		));

		let result = vec![42u8];
		let result_hash = frame::hashing::BlakeTwo256::hash(&result);
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, result_hash));

		// Move past commit deadline
		frame_system::Pallet::<Test>::set_block_number(12);

		// Try to reveal with different result
		let wrong_result = vec![99u8];
		assert_noop!(
			Agora::reveal_result(RuntimeOrigin::signed(2), 0, wrong_result),
			Error::<Test>::CommitMismatch
		);
	});
}

#[test]
fn finalize_job_works() {
	new_test_ext().execute_with(|| {
		// Setup: Register workers, submit job
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 100));
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(3), 100));
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			vec![1, 2, 3],
			100
		));

		// Both workers commit same result
		let result = vec![42u8];
		let result_hash = frame::hashing::BlakeTwo256::hash(&result);
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, result_hash));
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(3), 0, result_hash));

		// Move past commit deadline
		frame_system::Pallet::<Test>::set_block_number(12);

		// Both workers reveal
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(2), 0, result.clone()));
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(3), 0, result.clone()));

		// Move past reveal deadline
		frame_system::Pallet::<Test>::set_block_number(23);

		// Finalize job
		assert_ok!(Agora::finalize_job(RuntimeOrigin::signed(1), 0));

		// Check job is completed
		let job = Agora::jobs(0).unwrap();
		assert_eq!(job.status, crate::types::JobStatus::Completed);

		// Check result is stored
		let stored_result = Agora::results(0).unwrap();
		assert_eq!(stored_result.to_vec(), result);

		// Check workers were rewarded
		let worker2_info = Agora::workers(2).unwrap();
		assert_eq!(worker2_info.reputation, 510); // 500 + 10

		let worker3_info = Agora::workers(3).unwrap();
		assert_eq!(worker3_info.reputation, 510); // 500 + 10
	});
}

#[test]
fn finalize_job_slashes_dishonest_workers() {
	new_test_ext().execute_with(|| {
		// Setup: Register 3 workers, submit job
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 100));
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(3), 100));
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(4), 100));
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			vec![1, 2, 3],
			100
		));

		// 2 workers commit correct result, 1 commits wrong result
		let correct_result = vec![42u8];
		let wrong_result = vec![99u8];
		let correct_hash = frame::hashing::BlakeTwo256::hash(&correct_result);
		let wrong_hash = frame::hashing::BlakeTwo256::hash(&wrong_result);

		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, correct_hash));
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(3), 0, correct_hash));
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(4), 0, wrong_hash));

		// Move past commit deadline
		frame_system::Pallet::<Test>::set_block_number(12);

		// Workers reveal
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(2), 0, correct_result.clone()));
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(3), 0, correct_result));
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(4), 0, wrong_result));

		// Move past reveal deadline
		frame_system::Pallet::<Test>::set_block_number(23);

		// Finalize job
		assert_ok!(Agora::finalize_job(RuntimeOrigin::signed(1), 0));

		// Check honest workers were rewarded
		let worker2_info = Agora::workers(2).unwrap();
		assert_eq!(worker2_info.reputation, 510); // 500 + 10
		
		let worker3_info = Agora::workers(3).unwrap();
		assert_eq!(worker3_info.reputation, 510); // 500 + 10

		// Check dishonest worker was slashed
		let worker4_info = Agora::workers(4).unwrap();
		assert_eq!(worker4_info.stake, 90); // 100 - 10% = 90
		assert_eq!(worker4_info.reputation, 450); // 500 - 50
	});
}

#[test]
fn multiple_jobs_work_independently() {
	new_test_ext().execute_with(|| {
		// Register worker
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 200));

		// Submit two jobs
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			vec![1],
			100
		));
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			0,
			vec![2],
			100
		));

		// Check both jobs exist
		assert!(Agora::jobs(0).is_some());
		assert!(Agora::jobs(1).is_some());

		// Check next job ID incremented
		assert_eq!(Agora::next_job_id(), 2);
	});
}

