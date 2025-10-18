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
fn simple_salt_verification_test() {
	new_test_ext().execute_with(|| {
		// Simple test with explicit values
		println!("=== SIMPLE SALT VERIFICATION TEST ===");
		
		// 1. Register worker
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 100));
		println!("✓ Worker registered");
		
		// 2. Submit job
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1, // Computation
			vec![72, 101, 108, 108, 111], // "Hello" in bytes
			100
		));
		println!("✓ Job submitted");
		
		// 3. Prepare commit with simple values
		let result = vec![67, 111, 109, 112, 117, 116, 101, 100]; // "Computed" in bytes
		let salt = [1u8; 32]; // Simple salt: all 1s
		
		// 4. Calculate hash manually
		let mut salted_input = Vec::new();
		salted_input.extend_from_slice(&salt);
		salted_input.extend_from_slice(&result);
		let result_hash = frame::hashing::BlakeTwo256::hash(&salted_input);
		
		println!("Salt: {:?}", salt);
		println!("Result: {:?}", result);
		println!("Salted input length: {}", salted_input.len());
		println!("Salted input: {:?}", salted_input);
		println!("Calculated hash: {:?}", result_hash);
		
		// 5. Commit result
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, salt, result_hash));
		println!("✓ Result committed");
		
		// 6. Move past commit deadline
		frame_system::Pallet::<Test>::set_block_number(32);
		println!("✓ Moved past commit deadline");
		
		// 7. Reveal result
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(2), 0, result.clone()));
		println!("✓ Result revealed");
		
		// 8. Check reveal was stored
		let reveals = Agora::reveals(0).unwrap();
		assert_eq!(reveals.len(), 1);
		assert_eq!(reveals[0].worker, 2);
		assert_eq!(reveals[0].result.to_vec(), result);
		println!("✓ Reveal stored correctly");
		
		println!("=== TEST PASSED ===");
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
		let salt = [1u8; 32]; // 32-byte salt
		let result_hash = frame::hashing::BlakeTwo256::hash(&result);

		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, salt, result_hash));

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
		let salt = [1u8; 32]; // 32-byte salt
		let result_hash = frame::hashing::BlakeTwo256::hash(&[42u8]);
		assert_noop!(
			Agora::commit_result(RuntimeOrigin::signed(2), 0, salt, result_hash),
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
		let salt = [1u8; 32]; // 32-byte salt
		let mut salted_input = Vec::new();
		salted_input.extend_from_slice(&salt);
		salted_input.extend_from_slice(&result);
		let result_hash = frame::hashing::BlakeTwo256::hash(&salted_input);
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, salt, result_hash));

		// Move past commit deadline
		frame_system::Pallet::<Test>::set_block_number(32);

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
		let salt = [1u8; 32]; // 32-byte salt
		let mut salted_input = Vec::new();
		salted_input.extend_from_slice(&salt);
		salted_input.extend_from_slice(&result);
		let result_hash = frame::hashing::BlakeTwo256::hash(&salted_input);
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, salt, result_hash));

		// Move past commit deadline
		frame_system::Pallet::<Test>::set_block_number(32);

		// Try to reveal with different result
		let wrong_result = vec![99u8];
		assert_noop!(
			Agora::reveal_result(RuntimeOrigin::signed(2), 0, wrong_result),
			Error::<Test>::SaltVerificationFailed
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
		let salt1 = [1u8; 32]; // Worker 2's salt
		let salt2 = [2u8; 32]; // Worker 3's salt
		
		let mut salted_input1 = Vec::new();
		salted_input1.extend_from_slice(&salt1);
		salted_input1.extend_from_slice(&result);
		let result_hash1 = frame::hashing::BlakeTwo256::hash(&salted_input1);
		
		let mut salted_input2 = Vec::new();
		salted_input2.extend_from_slice(&salt2);
		salted_input2.extend_from_slice(&result);
		let result_hash2 = frame::hashing::BlakeTwo256::hash(&salted_input2);
		
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, salt1, result_hash1));
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(3), 0, salt2, result_hash2));

		// Move past commit deadline
		frame_system::Pallet::<Test>::set_block_number(32);

		// Both workers reveal
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(2), 0, result.clone()));
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(3), 0, result.clone()));

		// Move past reveal deadline
		frame_system::Pallet::<Test>::set_block_number(62);

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
		
		let salt1 = [1u8; 32]; // Worker 2's salt
		let salt2 = [2u8; 32]; // Worker 3's salt
		let salt3 = [3u8; 32]; // Worker 4's salt
		
		let mut salted_input1 = Vec::new();
		salted_input1.extend_from_slice(&salt1);
		salted_input1.extend_from_slice(&correct_result);
		let correct_hash1 = frame::hashing::BlakeTwo256::hash(&salted_input1);
		
		let mut salted_input2 = Vec::new();
		salted_input2.extend_from_slice(&salt2);
		salted_input2.extend_from_slice(&correct_result);
		let correct_hash2 = frame::hashing::BlakeTwo256::hash(&salted_input2);
		
		let mut salted_input3 = Vec::new();
		salted_input3.extend_from_slice(&salt3);
		salted_input3.extend_from_slice(&wrong_result);
		let wrong_hash = frame::hashing::BlakeTwo256::hash(&salted_input3);

		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, salt1, correct_hash1));
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(3), 0, salt2, correct_hash2));
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(4), 0, salt3, wrong_hash));

		// Move past commit deadline
		frame_system::Pallet::<Test>::set_block_number(32);

		// Workers reveal
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(2), 0, correct_result.clone()));
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(3), 0, correct_result));
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(4), 0, wrong_result));

		// Move past reveal deadline
		frame_system::Pallet::<Test>::set_block_number(62);

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

#[test]
fn calculate_verified_hashes_for_polkadot_js() {
	new_test_ext().execute_with(|| {
		println!("\n=== VERIFIED HASHES FOR POLKADOT.JS ===\n");
		
		// Correct result that Bob and Charlie will use
		let result_correct = vec![0x2a, 0x54, 0x7e, 0xa8, 0xd2];
		println!("Correct Result: 0x2a547ea8d2");
		
		// Wrong result that Dave will use
		let result_wrong = vec![0x63, 0x63, 0x63, 0x63, 0x63];
		println!("Wrong Result: 0x6363636363\n");
		
		// Bob's values
		let salt_bob: [u8; 32] = [
			0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80,
			0x90, 0xa0, 0xb0, 0xc0, 0xd0, 0xe0, 0xf0, 0x00,
			0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
			0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x01
		];
		
		let mut salted_input_bob = Vec::new();
		salted_input_bob.extend_from_slice(&salt_bob);
		salted_input_bob.extend_from_slice(&result_correct);
		let hash_bob = frame::hashing::BlakeTwo256::hash(&salted_input_bob);
		
		println!("BOB (Honest Worker):");
		println!("  salt: 0x102030405060708090a0b0c0d0e0f000112233445566778899aabbccddeeff01");
		println!("  resultHash: 0x{:x}", hash_bob);
		println!();
		
		// Charlie's values
		let salt_charlie: [u8; 32] = [
			0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90,
			0xa0, 0xb0, 0xc0, 0xd0, 0xe0, 0xf0, 0x00, 0x10,
			0x21, 0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98,
			0xa9, 0xba, 0xcb, 0xdc, 0xed, 0xfe, 0x0f, 0x11
		];
		
		let mut salted_input_charlie = Vec::new();
		salted_input_charlie.extend_from_slice(&salt_charlie);
		salted_input_charlie.extend_from_slice(&result_correct);
		let hash_charlie = frame::hashing::BlakeTwo256::hash(&salted_input_charlie);
		
		println!("CHARLIE (Honest Worker):");
		println!("  salt: 0x2030405060708090a0b0c0d0e0f000102132435465768798a9bacbdcdefe0f11");
		println!("  resultHash: 0x{:x}", hash_charlie);
		println!();
		
		// Dave's values
		let salt_dave: [u8; 32] = [
			0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0,
			0xb0, 0xc0, 0xd0, 0xe0, 0xf0, 0x00, 0x10, 0x20,
			0x31, 0x42, 0x53, 0x64, 0x75, 0x86, 0x97, 0xa8,
			0xb9, 0xca, 0xdb, 0xec, 0xfd, 0x0e, 0x1f, 0x21
		];
		
		let mut salted_input_dave = Vec::new();
		salted_input_dave.extend_from_slice(&salt_dave);
		salted_input_dave.extend_from_slice(&result_wrong);
		let hash_dave = frame::hashing::BlakeTwo256::hash(&salted_input_dave);
		
		println!("DAVE (Dishonest Worker):");
		println!("  salt: 0x30405060708090a0b0c0d0e0f000102031425364758697a8b9cadbecfd0e1f21");
		println!("  resultHash: 0x{:x}", hash_dave);
		println!();
		
		println!("=== COPY/PASTE FOR POLKADOT.JS ===\n");
		println!("REVEAL PHASE:");
		println!("Bob: result: 0x2a547ea8d2");
		println!("Charlie: result: 0x2a547ea8d2");
		println!("Dave: result: 0x6363636363");
		println!();
		
		// Now verify each one manually
		println!("=== VERIFICATION ===");
		
		// Verify Bob
		let mut verify_bob = Vec::new();
		verify_bob.extend_from_slice(&salt_bob);
		verify_bob.extend_from_slice(&result_correct);
		let verify_hash_bob = frame::hashing::BlakeTwo256::hash(&verify_bob);
		println!("Bob verification: hash matches = {}", verify_hash_bob == hash_bob);
		
		// Verify Charlie
		let mut verify_charlie = Vec::new();
		verify_charlie.extend_from_slice(&salt_charlie);
		verify_charlie.extend_from_slice(&result_correct);
		let verify_hash_charlie = frame::hashing::BlakeTwo256::hash(&verify_charlie);
		println!("Charlie verification: hash matches = {}", verify_hash_charlie == hash_charlie);
		
		// Verify Dave
		let mut verify_dave = Vec::new();
		verify_dave.extend_from_slice(&salt_dave);
		verify_dave.extend_from_slice(&result_wrong);
		let verify_hash_dave = frame::hashing::BlakeTwo256::hash(&verify_dave);
		println!("Dave verification: hash matches = {}", verify_hash_dave == hash_dave);
	});
}

#[test]
fn test_actual_commit_reveal_from_polkadot_js() {
	new_test_ext().execute_with(|| {
		println!("\n=== TESTING ACTUAL POLKADOT.JS VALUES ===\n");
		
		// Register workers
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 200));
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(3), 200));
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(4), 200));
		
		// Submit job
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			vec![72, 101, 108, 108, 111], // "Hello"
			150
		));
		
		// These are the ACTUAL salts and hashes from Polkadot.js
		let salt_bob: [u8; 32] = [
			0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80,
			0x90, 0xa0, 0xb0, 0xc0, 0xd0, 0xe0, 0xf0, 0x00,
			0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
			0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x01
		];
		
		let salt_charlie: [u8; 32] = [
			0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90,
			0xa0, 0xb0, 0xc0, 0xd0, 0xe0, 0xf0, 0x00, 0x10,
			0x21, 0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98,
			0xa9, 0xba, 0xcb, 0xdc, 0xed, 0xfe, 0x0f, 0x11
		];
		
		let salt_dave: [u8; 32] = [
			0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0,
			0xb0, 0xc0, 0xd0, 0xe0, 0xf0, 0x00, 0x10, 0x20,
			0x31, 0x42, 0x53, 0x64, 0x75, 0x86, 0x97, 0xa8,
			0xb9, 0xca, 0xdb, 0xec, 0xfd, 0x0e, 0x1f, 0x21
		];
		
		// Calculate hashes
		let result_correct = vec![0x2a, 0x54, 0x7e, 0xa8, 0xd2];
		let result_wrong = vec![0x63, 0x63, 0x63, 0x63, 0x63];
		
		let mut salted_bob = Vec::new();
		salted_bob.extend_from_slice(&salt_bob);
		salted_bob.extend_from_slice(&result_correct);
		let hash_bob = frame::hashing::BlakeTwo256::hash(&salted_bob);
		
		let mut salted_charlie = Vec::new();
		salted_charlie.extend_from_slice(&salt_charlie);
		salted_charlie.extend_from_slice(&result_correct);
		let hash_charlie = frame::hashing::BlakeTwo256::hash(&salted_charlie);
		
		let mut salted_dave = Vec::new();
		salted_dave.extend_from_slice(&salt_dave);
		salted_dave.extend_from_slice(&result_wrong);
		let hash_dave = frame::hashing::BlakeTwo256::hash(&salted_dave);
		
		println!("Bob hash: 0x{:x}", hash_bob);
		println!("Charlie hash: 0x{:x}", hash_charlie);
		println!("Dave hash: 0x{:x}", hash_dave);
		
		// Commit
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), 0, salt_bob, hash_bob));
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(3), 0, salt_charlie, hash_charlie));
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(4), 0, salt_dave, hash_dave));
		
		// Move past commit deadline
		frame_system::Pallet::<Test>::set_block_number(32);
		
		// Reveal - THIS SHOULD WORK
		println!("\nTrying to reveal with result: 0x2a547ea8d2");
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(2), 0, result_correct.clone()));
		println!("✅ Bob reveal SUCCESS");
		
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(3), 0, result_correct.clone()));
		println!("✅ Charlie reveal SUCCESS");
		
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(4), 0, result_wrong));
		println!("✅ Dave reveal SUCCESS");
	});
}

#[test]
fn check_what_dave_should_have_committed() {
	new_test_ext().execute_with(|| {
		println!("\n=== CHECKING DAVE'S ACTUAL COMMITMENT ===\n");
		
		let salt_dave: [u8; 32] = [
			0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0,
			0xb0, 0xc0, 0xd0, 0xe0, 0xf0, 0x00, 0x10, 0x20,
			0x31, 0x42, 0x53, 0x64, 0x75, 0x86, 0x97, 0xa8,
			0xb9, 0xca, 0xdb, 0xec, 0xfd, 0x0e, 0x1f, 0x21
		];
		
		// Dave committed: 0xdec1153d26522cc066a39e62ec75ea733d1da1e87aa711d5242b8292224817be
		// Let's check what result produces this hash
		
		// Try with correct result
		let result_correct = vec![0x2a, 0x54, 0x7e, 0xa8, 0xd2];
		let mut salted_correct = Vec::new();
		salted_correct.extend_from_slice(&salt_dave);
		salted_correct.extend_from_slice(&result_correct);
		let hash_with_correct = frame::hashing::BlakeTwo256::hash(&salted_correct);
		println!("Dave's salt + 0x2a547ea8d2 = 0x{:x}", hash_with_correct);
		
		// Try with wrong result
		let result_wrong = vec![0x63, 0x63, 0x63, 0x63, 0x63];
		let mut salted_wrong = Vec::new();
		salted_wrong.extend_from_slice(&salt_dave);
		salted_wrong.extend_from_slice(&result_wrong);
		let hash_with_wrong = frame::hashing::BlakeTwo256::hash(&salted_wrong);
		println!("Dave's salt + 0x6363636363 = 0x{:x}", hash_with_wrong);
		
		println!("\nDave actually committed: 0xdec1153d26522cc066a39e62ec75ea733d1da1e87aa711d5242b8292224817be");
		println!("So Dave should reveal: 0x6363636363");
	});
}

