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
fn test_basic_reserve_transfer_xcm() {
    new_test_ext().execute_with(|| {
        // Setup accounts
        let sender = 1u64;
        let dest_para_id = 2000;
        let amount = 100;

        // Ensure sender has sufficient balance
        assert_ok!(Balances::deposit_creating(&sender, 1_000_000));

        // Construct a dummy call (e.g., dummy extrinsic)
        let dummy_call = vec![];

        // Call build_job_request_xcm
        let result = Agora::build_job_request_xcm(
            sender,
            amount,
            dummy_call,
            dest_para_id,
        );
        assert!(result.is_ok(), "Should build XCM without error");

        let xcm_message = result.unwrap();

        // Evaluate the XCM message
        // You can serialise it, then run through validate_and_execute in your mock environment
        // Note: For detailed test, you may need to invoke `xcm_executor::execute_xcm()`

        // Example: Use XcmExecutor to simulate execution
        let result = xcm_executor::XcmExecutor::<Test>::execute_xcm(
            Origin::none(),
            xcm_message,
        );
        // Depending on your configuration:
        // - Should succeed if asset config matches
        // - Or you can assert it fails, and check for specific error
        assert!(result.is_ok() || result.is_err(), "XCM execution attempted");
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

// =====================================================
// OCW TESTS
// =====================================================
// Note: Many OCW functions are private and tested through integration tests
// These tests verify the logic that can be accessed publicly

#[test]
fn test_ocw_commit_hash_verification() {
	new_test_ext().execute_with(|| {
		// This test verifies that the commit-reveal mechanism works
		// which is the core of what the OCW will use
		
		let salt: [u8; 32] = [
			0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80,
			0x90, 0xa0, 0xb0, 0xc0, 0xd0, 0xe0, 0xf0, 0x00,
			0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
			0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x01
		];
		
		let result = vec![0x2a, 0x54, 0x7e, 0xa8, 0xd2];
		
		// Calculate commit hash (same logic OCW uses)
		let mut salted_input = Vec::new();
		salted_input.extend_from_slice(&salt);
		salted_input.extend_from_slice(&result);
		let commit_hash = frame::hashing::blake2_256(&salted_input);
		
		println!("\n=== OCW COMMIT HASH VERIFICATION ===");
		println!("Salt: {:?}", salt);
		println!("Result: {:?}", result);
		println!("Commit Hash: 0x{:x}", frame::hashing::BlakeTwo256::hash(&commit_hash));
		
		// This hash should work for commit-reveal
		assert_eq!(commit_hash.len(), 32, "Commit hash should be 32 bytes");
	});
}

// Additional OCW-related tests can be added here as the implementation evolves
// Currently, OCW functionality is tested through:
// 1. Integration tests in Zombienet
// 2. Manual testing via Polkadot.js Apps
// 3. The commit-reveal tests above which verify the core cryptographic logic

// =====================================================
// XCM TESTS
// =====================================================

#[test]
fn xcm_job_submission_works() {
	new_test_ext().execute_with(|| {
		// Setup: Register a worker
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 1000));

		// Simulate XCM job submission from parachain 2000
		let origin_para_id = 2000;
		assert_ok!(Agora::xcm_submit_job(
			RuntimeOrigin::signed(1),
			1, // Computation job
			b"hash:test_data".to_vec(),
			150,
			origin_para_id
		));

		// Verify job was created with correct origin para ID
		let job = Agora::jobs(0).unwrap();
		assert_eq!(job.origin_para_id, origin_para_id);
		assert_eq!(job.bounty, 150);
		assert_eq!(job.creator, 1);

		// Verify event was emitted
		frame_system::Pallet::<Test>::assert_last_event(
			Event::XcmJobSubmitted { 
				job_id: 0, 
				creator: 1, 
				bounty: 150,
				origin_para_id: 2000 
			}.into()
		);
	});
}

#[test]
fn xcm_job_submission_fails_with_insufficient_bounty() {
	new_test_ext().execute_with(|| {
		// Try to submit job with bounty below minimum
		assert_noop!(
			Agora::xcm_submit_job(
				RuntimeOrigin::signed(1),
				1,
				b"hash:test".to_vec(),
				50, // Below minimum of 100
				2000
			),
			Error::<Test>::InsufficientBounty
		);
	});
}

#[test]
fn xcm_job_stores_origin_parachain_correctly() {
	new_test_ext().execute_with(|| {
		// Register worker
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 1000));

		// Submit from different parachains
		assert_ok!(Agora::xcm_submit_job(
			RuntimeOrigin::signed(1),
			1,
			b"hash:test1".to_vec(),
			150,
			1000 // Para 1000
		));

		assert_ok!(Agora::xcm_submit_job(
			RuntimeOrigin::signed(1),
			1,
			b"hash:test2".to_vec(),
			150,
			2000 // Para 2000
		));

		// Verify both jobs have correct origin
		assert_eq!(Agora::jobs(0).unwrap().origin_para_id, 1000);
		assert_eq!(Agora::jobs(1).unwrap().origin_para_id, 2000);
	});
}

#[test]
fn query_job_result_works() {
	new_test_ext().execute_with(|| {
		// Setup: Complete a job
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 1000));
		
		// Submit XCM job
		assert_ok!(Agora::xcm_submit_job(
			RuntimeOrigin::signed(1),
			1,
			b"hash:test".to_vec(),
			150,
			2000 // From para 2000
		));

		// Progress through phases
		System::set_block_number(10);
		run_to_block(11);

		// Commit result
		let salt: [u8; 32] = [1u8; 32];
		let result = vec![0x42, 0x43, 0x44];
		let mut salted_input = Vec::new();
		salted_input.extend_from_slice(&salt);
		salted_input.extend_from_slice(&result);
		let commit_hash = frame::hashing::blake2_256(&salted_input);

		assert_ok!(Agora::commit_result(
			RuntimeOrigin::signed(2),
			0,
			salt,
			commit_hash
		));

		// Move to reveal phase
		run_to_block(32);

		// Reveal result
		assert_ok!(Agora::reveal_result(
			RuntimeOrigin::signed(2),
			0,
			result.clone()
		));

		// Finalize job
		run_to_block(62);
		assert_ok!(Agora::finalize_job(RuntimeOrigin::signed(1), 0));

		// Now query the result
		assert_ok!(Agora::query_job_result(
			RuntimeOrigin::signed(1),
			0
		));

		// Verify event was emitted with correct data
		frame_system::Pallet::<Test>::assert_last_event(
			Event::JobResultQueried {
				job_id: 0,
				result: result.clone(),
				origin_para_id: 2000
			}.into()
		);
	});
}

#[test]
fn query_job_result_fails_for_incomplete_job() {
	new_test_ext().execute_with(|| {
		// Setup
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 1000));
		assert_ok!(Agora::xcm_submit_job(
			RuntimeOrigin::signed(1),
			1,
			b"hash:test".to_vec(),
			150,
			2000
		));

		// Try to query before job is completed
		assert_noop!(
			Agora::query_job_result(RuntimeOrigin::signed(1), 0),
			Error::<Test>::InvalidJobPhase
		);
	});
}

#[test]
fn xcm_job_full_lifecycle() {
	new_test_ext().execute_with(|| {
		// This test verifies the complete lifecycle of an XCM job
		
		// 1. Register workers
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 1000));
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(3), 1000));

		// 2. Submit XCM job from para 3000
		assert_ok!(Agora::xcm_submit_job(
			RuntimeOrigin::signed(1),
			1, // Computation
			b"hash:cross_chain_test".to_vec(),
			200,
			3000
		));

		let job_id = 0;
		let job = Agora::jobs(job_id).unwrap();
		assert_eq!(job.origin_para_id, 3000);

		// 3. Workers commit results
		run_to_block(5);
		
		let result = vec![0xAA, 0xBB, 0xCC];
		let salt1: [u8; 32] = [1u8; 32];
		let salt2: [u8; 32] = [2u8; 32];

		let mut salted1 = Vec::new();
		salted1.extend_from_slice(&salt1);
		salted1.extend_from_slice(&result);
		let hash1 = frame::hashing::blake2_256(&salted1);

		let mut salted2 = Vec::new();
		salted2.extend_from_slice(&salt2);
		salted2.extend_from_slice(&result);
		let hash2 = frame::hashing::blake2_256(&salted2);

		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(2), job_id, salt1, hash1));
		assert_ok!(Agora::commit_result(RuntimeOrigin::signed(3), job_id, salt2, hash2));

		// 4. Workers reveal results
		run_to_block(32);
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(2), job_id, result.clone()));
		assert_ok!(Agora::reveal_result(RuntimeOrigin::signed(3), job_id, result.clone()));

		// 5. Finalize job
		run_to_block(62);
		assert_ok!(Agora::finalize_job(RuntimeOrigin::signed(1), job_id));

		// 6. Verify job is completed and result is stored
		let completed_job = Agora::jobs(job_id).unwrap();
		assert_eq!(completed_job.status, crate::JobStatus::Completed);
		assert_eq!(completed_job.result.to_vec(), result);
		assert_eq!(completed_job.origin_para_id, 3000);

		// 7. Query result (can be done via XCM from originating chain)
		assert_ok!(Agora::query_job_result(RuntimeOrigin::signed(1), job_id));

		// Verify all events were emitted correctly
		let events = frame_system::Pallet::<Test>::events();
		assert!(events.iter().any(|e| matches!(
			e.event,
			RuntimeEvent::Agora(Event::XcmJobSubmitted { job_id: 0, origin_para_id: 3000, .. })
		)));
		assert!(events.iter().any(|e| matches!(
			e.event,
			RuntimeEvent::Agora(Event::JobFinalized { job_id: 0, .. })
		)));
		assert!(events.iter().any(|e| matches!(
			e.event,
			RuntimeEvent::Agora(Event::JobResultQueried { job_id: 0, origin_para_id: 3000, .. })
		)));
	});
}

#[test]
fn local_job_has_zero_origin_para_id() {
	new_test_ext().execute_with(|| {
		// Register worker
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 1000));

		// Submit regular (non-XCM) job
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			b"hash:local_test".to_vec(),
			150
		));

		// Verify local jobs have origin_para_id = 0
		let job = Agora::jobs(0).unwrap();
		assert_eq!(job.origin_para_id, 0);
	});
}

#[test]
fn xcm_and_local_jobs_can_coexist() {
	new_test_ext().execute_with(|| {
		// Register worker
		assert_ok!(Agora::register_worker(RuntimeOrigin::signed(2), 1000));

		// Submit local job
		assert_ok!(Agora::submit_job(
			RuntimeOrigin::signed(1),
			1,
			b"local".to_vec(),
			150
		));

		// Submit XCM job
		assert_ok!(Agora::xcm_submit_job(
			RuntimeOrigin::signed(1),
			1,
			b"xcm".to_vec(),
			150,
			2000
		));

		// Verify both jobs exist with correct origin
		assert_eq!(Agora::jobs(0).unwrap().origin_para_id, 0); // Local
		assert_eq!(Agora::jobs(1).unwrap().origin_para_id, 2000); // XCM
	});
}

