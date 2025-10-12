//! Benchmarking setup for pallet-agora
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as AgoraPallet;
use frame::benchmarking::prelude::*;
use frame::prelude::*;
use frame::traits::fungible::*;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn submit_job() {
		let caller: T::AccountId = whitelisted_caller();
		let initial_balance = 10000u128;
		
		// Fund the caller
		T::Currency::mint_into(&caller, initial_balance).unwrap();

		let input_data = vec![1u8; 100];
		let bounty = 100u128;

		#[extrinsic_call]
		submit_job(
			RawOrigin::Signed(caller),
			crate::types::JobType::Computation,
			input_data,
			bounty,
		);

		assert_eq!(AgoraPallet::<T>::next_job_id(), 1);
	}

	#[benchmark]
	fn register_worker() {
		let caller: T::AccountId = whitelisted_caller();
		let initial_balance = 10000u128;
		
		// Fund the caller
		T::Currency::mint_into(&caller, initial_balance).unwrap();

		let stake = 100u128;

		#[extrinsic_call]
		register_worker(RawOrigin::Signed(caller.clone()), stake);

		assert!(AgoraPallet::<T>::workers(&caller).is_some());
	}

	#[benchmark]
	fn unregister_worker() {
		let caller: T::AccountId = whitelisted_caller();
		let initial_balance = 10000u128;
		
		// Fund and register the caller
		T::Currency::mint_into(&caller, initial_balance).unwrap();
		AgoraPallet::<T>::register_worker(RawOrigin::Signed(caller.clone()).into(), 100u128)
			.unwrap();

		#[extrinsic_call]
		unregister_worker(RawOrigin::Signed(caller.clone()));

		assert!(AgoraPallet::<T>::workers(&caller).is_none());
	}

	#[benchmark]
	fn commit_result() {
		let caller: T::AccountId = whitelisted_caller();
		let job_creator: T::AccountId = account("creator", 0, 0);
		let initial_balance = 10000u128;
		
		// Fund accounts
		T::Currency::mint_into(&caller, initial_balance).unwrap();
		T::Currency::mint_into(&job_creator, initial_balance).unwrap();

		// Register worker
		AgoraPallet::<T>::register_worker(RawOrigin::Signed(caller.clone()).into(), 100u128)
			.unwrap();

		// Submit job
		AgoraPallet::<T>::submit_job(
			RawOrigin::Signed(job_creator).into(),
			crate::types::JobType::Computation,
			vec![1, 2, 3],
			100u128,
		)
		.unwrap();

		let result_hash = T::Hashing::hash(&[42u8]);

		#[extrinsic_call]
		commit_result(RawOrigin::Signed(caller), 0, result_hash);

		assert!(AgoraPallet::<T>::commits(0).is_some());
	}

	#[benchmark]
	fn reveal_result() {
		let caller: T::AccountId = whitelisted_caller();
		let job_creator: T::AccountId = account("creator", 0, 0);
		let initial_balance = 10000u128;
		
		// Fund accounts
		T::Currency::mint_into(&caller, initial_balance).unwrap();
		T::Currency::mint_into(&job_creator, initial_balance).unwrap();

		// Register worker
		AgoraPallet::<T>::register_worker(RawOrigin::Signed(caller.clone()).into(), 100u128)
			.unwrap();

		// Submit job
		AgoraPallet::<T>::submit_job(
			RawOrigin::Signed(job_creator).into(),
			crate::types::JobType::Computation,
			vec![1, 2, 3],
			100u128,
		)
		.unwrap();

		// Commit result
		let result = vec![42u8];
		let result_hash = T::Hashing::hash(&result);
		AgoraPallet::<T>::commit_result(RawOrigin::Signed(caller.clone()).into(), 0, result_hash)
			.unwrap();

		// Move past commit deadline
		frame_system::Pallet::<T>::set_block_number(20u32.into());

		#[extrinsic_call]
		reveal_result(RawOrigin::Signed(caller), 0, result);

		assert!(AgoraPallet::<T>::reveals(0).is_some());
	}

	#[benchmark]
	fn finalize_job() {
		let worker1: T::AccountId = account("worker1", 0, 0);
		let worker2: T::AccountId = account("worker2", 0, 1);
		let job_creator: T::AccountId = account("creator", 0, 2);
		let caller: T::AccountId = whitelisted_caller();
		let initial_balance = 10000u128;
		
		// Fund accounts
		T::Currency::mint_into(&worker1, initial_balance).unwrap();
		T::Currency::mint_into(&worker2, initial_balance).unwrap();
		T::Currency::mint_into(&job_creator, initial_balance).unwrap();

		// Register workers
		AgoraPallet::<T>::register_worker(RawOrigin::Signed(worker1.clone()).into(), 100u128)
			.unwrap();
		AgoraPallet::<T>::register_worker(RawOrigin::Signed(worker2.clone()).into(), 100u128)
			.unwrap();

		// Submit job
		AgoraPallet::<T>::submit_job(
			RawOrigin::Signed(job_creator).into(),
			crate::types::JobType::Computation,
			vec![1, 2, 3],
			100u128,
		)
		.unwrap();

		// Workers commit
		let result = vec![42u8];
		let result_hash = T::Hashing::hash(&result);
		AgoraPallet::<T>::commit_result(RawOrigin::Signed(worker1.clone()).into(), 0, result_hash)
			.unwrap();
		AgoraPallet::<T>::commit_result(RawOrigin::Signed(worker2.clone()).into(), 0, result_hash)
			.unwrap();

		// Move past commit deadline
		frame_system::Pallet::<T>::set_block_number(20u32.into());

		// Workers reveal
		AgoraPallet::<T>::reveal_result(
			RawOrigin::Signed(worker1).into(),
			0,
			result.clone(),
		)
		.unwrap();
		AgoraPallet::<T>::reveal_result(RawOrigin::Signed(worker2).into(), 0, result).unwrap();

		// Move past reveal deadline
		frame_system::Pallet::<T>::set_block_number(40u32.into());

		#[extrinsic_call]
		finalize_job(RawOrigin::Signed(caller), 0);

		assert!(AgoraPallet::<T>::results(0).is_some());
	}

	impl_benchmark_test_suite!(AgoraPallet, crate::mock::new_test_ext(), crate::mock::Test);
}


