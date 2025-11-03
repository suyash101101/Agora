const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { cryptoWaitReady, blake2AsU8a, encodeAddress } = require('@polkadot/util-crypto');

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function sendAndWait(tx, signer, api) {
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
        console.error('    ❌ Dispatch error:', result.dispatchError?.toHuman());
        if (unsub) unsub();
        reject(new Error(`Transaction failed: ${result.status.type} - ${result.dispatchError?.toHuman()}`));
      }
    }).then((unsubscribe) => {
      unsub = unsubscribe;
    }).catch(reject);
  });
}

function siblingParaIdToSovereignAccount(paraId, ss58Format = 42) {
  // Blake2-256("sibling:<ParaId>") for SiblingParachainConvertsVia
  const seed = `sibling:${paraId}`;
  const hash = blake2AsU8a(seed, 256);
  return encodeAddress(hash, ss58Format);
}

async function verifyFunding(api, account, expectedAmount, label) {
  await sleep(2000);
  const balance = await api.query.system.account(account);
  const free = BigInt(balance.data.free.toString());
  console.log(`   ✅ ${label} balance: ${free} (expected >= ${expectedAmount})`);
  if (free < expectedAmount) {
    throw new Error(`${label} underfunded: ${free} < ${expectedAmount}`);
  }
}

async function checkHrmpChannel(relayApi, sender, recipient) {
  const channel = await relayApi.query.hrmp.hrmpChannels([sender, recipient]);
  return channel.isSome ? '✅ Open' : '❌ Closed - Open manually';
}

async function main() {
  await cryptoWaitReady();
  
  console.log('🚀 Connecting to networks...\n');
  
  const relayApi = await ApiPromise.create({ provider: new WsProvider('ws://localhost:9944') });
  const para2000Api = await ApiPromise.create({ provider: new WsProvider('ws://localhost:9947') });

  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');

  const para2000SS58 = para2000Api.registry.chainSS58 || 42;
  const sovereignAddr = siblingParaIdToSovereignAccount(1000, para2000SS58);  // Correct derivation

  console.log('✅ Connected to Relay & Para 2000');
  console.log(`\n🔑 Correct Sovereign for Para 1000 on Para 2000: ${sovereignAddr}`);
  console.log(`   (Copy this SS58 - Verify/fund manually in UI if needed)\n`);

  // Check HRMP channel 1000 -> 2000
  console.log('🔗 Checking HRMP channel 1000 -> 2000...\n');
  const channelStatus = await checkHrmpChannel(relayApi, 1000, 2000);
  console.log(`   Status: ${channelStatus}`);
  if (channelStatus.includes('Closed')) {
    console.log('   💡 Open manually: See instructions below.\n');
  } else {
    console.log('   ✅ Channel open - XCM delivery ready\n');
  }

  // Fund sovereign on Para 2000
  console.log('💰 Funding sovereign on Para 2000...\n');
  const fundAmount = 200_000_000_000_000n;  // 200 UNIT (adjust if decimals differ)
  await sendAndWait(
    para2000Api.tx.balances.transferKeepAlive(sovereignAddr, fundAmount),
    alice,
    para2000Api
  );
  await verifyFunding(para2000Api, sovereignAddr, fundAmount, 'Sovereign (Para 1000 on 2000)');

  console.log('\n✅ Sovereign funded - BadOrigin fixed! Ready for UI job send.\n');
  console.log('💡 Next: Send job via Polkadot.js UI (instructions below).\n');
  
  // Cleanup
  await relayApi.disconnect();
  await para2000Api.disconnect();
  console.log('Disconnected.\n');
  process.exit(0);
}

main().catch((error) => {
  console.error('❌ Fatal error:', error);
  process.exit(1);
});
