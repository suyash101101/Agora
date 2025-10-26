//! Benchmarking setup for pallet-xcm-job-client

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::{assert_ok, BoundedVec};
use frame_system::RawOrigin;
use sp_runtime::traits::Hash;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn request_remote_job() {
		let caller: T::AccountId = whitelisted_caller();
		let dest_para_id = 2000u32;
		let input: BoundedVec<u8, T::MaxInputBytes> = 
			BoundedVec::try_from(vec![1u8; 100]).unwrap();
		let bounty = 1000u32.into();
		let program_hash = T::Hashing::hash(b"program");

		// Fund the caller
		T::Currency::make_free_balance_be(&caller, 10000u32.into());

		#[extrinsic_call]
		_(
			RawOrigin::Signed(caller.clone()),
			dest_para_id,
			input,
			bounty,
			program_hash
		);

		assert_eq!(Pallet::<T>::job_nonce(), 1);
	}

	#[benchmark]
	fn receive_remote_job_result() {
		let caller: T::AccountId = whitelisted_caller();
		let dest_para_id = 2000u32;
		let input: BoundedVec<u8, T::MaxInputBytes> = 
			BoundedVec::try_from(vec![1u8; 100]).unwrap();
		let bounty = 1000u32.into();
		let program_hash = T::Hashing::hash(b"program");

		// Fund the caller
		T::Currency::make_free_balance_be(&caller, 10000u32.into());

		// Create a pending job
		assert_ok!(Pallet::<T>::request_remote_job(
			RawOrigin::Signed(caller.clone()).into(),
			dest_para_id,
			input,
			bounty,
			program_hash
		));

		let job_id = Pallet::<T>::generate_job_id(&caller, dest_para_id).unwrap();
		let result_hash = T::Hashing::hash(b"result");

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), job_id, result_hash, true);

		assert_eq!(Pallet::<T>::pending_jobs(job_id), None);
	}

	#[benchmark]
	fn cancel_remote_job() {
		let caller: T::AccountId = whitelisted_caller();
		let dest_para_id = 2000u32;
		let input: BoundedVec<u8, T::MaxInputBytes> = 
			BoundedVec::try_from(vec![1u8; 100]).unwrap();
		let bounty = 1000u32.into();
		let program_hash = T::Hashing::hash(b"program");

		// Fund the caller
		T::Currency::make_free_balance_be(&caller, 10000u32.into());

		// Create a pending job
		assert_ok!(Pallet::<T>::request_remote_job(
			RawOrigin::Signed(caller.clone()).into(),
			dest_para_id,
			input,
			bounty,
			program_hash
		));

		let job_id = Pallet::<T>::generate_job_id(&caller, dest_para_id).unwrap();

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), job_id);

		assert_eq!(Pallet::<T>::pending_jobs(job_id), None);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}