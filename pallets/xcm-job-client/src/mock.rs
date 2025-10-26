use crate as pallet_xcm_job_client;
use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU128, ConstU32, ConstU64, Everything},
};
use sp_runtime::{traits::IdentityLookup, BuildStorage};
use staging_xcm::prelude::*;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		XcmJobClient: pallet_xcm_job_client,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<u128>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
}

// Mock XCM sender
pub struct TestXcmSender;
impl SendXcm for TestXcmSender {
	type Ticket = ();

	fn validate(
		_destination: &mut Option<Location>,
		_message: &mut Option<Xcm<()>>,
	) -> SendResult<Self::Ticket> {
		Ok(((), Assets::new()))
	}

	fn deliver(_ticket: Self::Ticket) -> Result<XcmHash, SendError> {
		Ok([0; 32])
	}
}

// Mock XCM execute origin
pub struct EnsureXcm;
impl frame_support::traits::EnsureOrigin<RuntimeOrigin> for EnsureXcm {
	type Success = ();

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		Ok(())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

impl pallet_xcm_job_client::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type XcmSender = TestXcmSender;
	type MaxInputBytes = ConstU32<1024>;
	type WeightInfo = ();
	type XcmExecuteOrigin = EnsureXcm;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(1, 1000000),
			(2, 1000000),
			(3, 1000000),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}