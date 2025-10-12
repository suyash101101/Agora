use frame::{
	deps::{frame_support::weights::constants::RocksDbWeight, frame_system::GenesisConfig},
	prelude::*,
	runtime::prelude::*,
	testing_prelude::*,
	traits::fungible::Mutate,
};

// Configure a mock runtime to test the pallet.
#[frame_construct_runtime]
mod test_runtime {
	#[runtime::runtime]
	#[runtime::derive(
		RuntimeCall,
		RuntimeEvent,
		RuntimeError,
		RuntimeOrigin,
		RuntimeFreezeReason,
		RuntimeHoldReason,
		RuntimeSlashReason,
		RuntimeLockId,
		RuntimeTask
	)]
	pub struct Test;

	#[runtime::pallet_index(0)]
	pub type System = frame_system;
	#[runtime::pallet_index(1)]
	pub type Balances = pallet_balances;
	#[runtime::pallet_index(2)]
	pub type Agora = crate;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Nonce = u64;
	type Block = MockBlock<Test>;
	type BlockHashCount = ConstU64<250>;
	type DbWeight = RocksDbWeight;
	type AccountData = pallet_balances::AccountData<u128>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = u128;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type RuntimeHoldReason = RuntimeHoldReason;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WeightInfo = ();
	type CommitPhaseDuration = ConstU64<10>;
	type RevealPhaseDuration = ConstU64<10>;
	type MinWorkerStake = ConstU128<100>;
	type MinJobBounty = ConstU128<50>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> TestState {
	let mut ext = GenesisConfig::<Test>::default().build_storage().unwrap().into();
	let state: &mut TestState = &mut ext;
	state.execute_with(|| {
		System::set_block_number(1);
		// Fund test accounts
		let _ = Balances::mint_into(&1, 10000);
		let _ = Balances::mint_into(&2, 10000);
		let _ = Balances::mint_into(&3, 10000);
		let _ = Balances::mint_into(&4, 10000);
	});
	ext
}
