const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { cryptoWaitReady } = require('@polkadot/util-crypto');

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function sendAndWait(tx, signer) {
  return new Promise((resolve, reject) => {
    let unsub;
    tx.signAndSend(signer, (result) => {
      console.log(`    Transaction status: ${result.status.type}`);
      
      if (result.status.isFinalized) {
        console.log(`    ✅ Finalized in block: ${result.status.asFinalized.toHex()}`);
        if (unsub) unsub();
        resolve(result);
      }
      
      if (result.isError || result.status.isInvalid || result.status.isDropped) {
        const error = result.dispatchError?.toHuman();
        console.error('    ❌ Transaction failed:', error);
        if (unsub) unsub();
        reject(new Error(`Transaction failed: ${result.status.type} - ${JSON.stringify(error)}`));
      }
    }).then((unsubscribe) => {
      unsub = unsubscribe;
    }).catch(reject);
  });
}

async function checkBalance(api, address, label) {
  const balance = await api.query.system.account(address);
  const free = BigInt(balance.data.free.toString());
  const reserved = BigInt(balance.data.reserved.toString());
  console.log(`   ${label}:`);
  console.log(`     Free: ${free} (${(Number(free) / 1e12).toFixed(2)} UNIT)`);
  console.log(`     Reserved: ${reserved} (${(Number(reserved) / 1e12).toFixed(2)} UNIT)`);
  console.log(`     Total: ${free + reserved} (${(Number(free + reserved) / 1e12).toFixed(2)} UNIT)`);
  return free;
}

async function main() {
  await cryptoWaitReady();
  
  console.log('🚀 Funding test accounts on Para 1000...\n');
  
  // Connect to Para 1000 (where Bob is trying to register)
  const para1000Api = await ApiPromise.create({ 
    provider: new WsProvider('ws://127.0.0.1:9946') 
  });

  const keyring = new Keyring({ type: 'sr25519', ss58Format: 42 });
  const alice = keyring.addFromUri('//Alice');
  
  // Test account addresses (well-known Substrate addresses)
  const testAccounts = {
    alice: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
    bob: '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
    charlie: '5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y',
  };

  console.log('📋 Test Account Addresses:');
  Object.entries(testAccounts).forEach(([name, address]) => {
    console.log(`   ${name}: ${address}`);
  });
  console.log('');

  // Check current balances
  console.log('💰 Current Balances:\n');
  await checkBalance(para1000Api, testAccounts.alice, 'Alice');
  await checkBalance(para1000Api, testAccounts.bob, 'Bob');
  await checkBalance(para1000Api, testAccounts.charlie, 'Charlie');
  console.log('');

  // Fund accounts (500 UNIT each - enough for multiple registrations and transactions)
  const fundAmount = 500_000_000_000_000n; // 500 UNIT
  
  console.log(`\n💸 Funding accounts with ${fundAmount} (500 UNIT) each...\n`);

  // Fund Bob
  console.log('📤 Funding Bob...');
  try {
    await sendAndWait(
      para1000Api.tx.balances.transferKeepAlive(testAccounts.bob, fundAmount),
      alice
    );
    console.log('✅ Bob funded successfully\n');
  } catch (error) {
    console.error('❌ Failed to fund Bob:', error.message);
  }

  // Fund Charlie
  console.log('📤 Funding Charlie...');
  try {
    await sendAndWait(
      para1000Api.tx.balances.transferKeepAlive(testAccounts.charlie, fundAmount),
      alice
    );
    console.log('✅ Charlie funded successfully\n');
  } catch (error) {
    console.error('❌ Failed to fund Charlie:', error.message);
  }

  // Verify final balances
  console.log('\n💰 Final Balances:\n');
  await checkBalance(para1000Api, testAccounts.alice, 'Alice');
  await checkBalance(para1000Api, testAccounts.bob, 'Bob');
  await checkBalance(para1000Api, testAccounts.charlie, 'Charlie');

  console.log('\n✅ Funding complete!');
  console.log('💡 You can now register Bob as a worker in the UI.\n');
  
  await para1000Api.disconnect();
  process.exit(0);
}

main().catch((error) => {
  console.error('❌ Fatal error:', error);
  process.exit(1);
});

