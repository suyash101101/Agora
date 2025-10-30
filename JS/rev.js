const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { cryptoWaitReady } = require('@polkadot/util-crypto');

async function quickOpen() {
  await cryptoWaitReady();
  
  const relayApi = await ApiPromise.create({ 
    provider: new WsProvider('ws://localhost:9944') 
  });
  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');

  console.log('Opening channel 2000 -> 1000 via relay chain sudo...\n');

  // Force open directly on relay chain
  await relayApi.tx.sudo.sudo(
    relayApi.tx.hrmp.forceOpenHrmpChannel(2000, 1000, 8, 8192)
  ).signAndSend(alice, ({ status }) => {
    console.log(`Status: ${status.type}`);
    if (status.isFinalized) {
      console.log('✅ Channel force-opened!');
      console.log('Run: node debug.js to verify');
      process.exit(0);
    }
  });
}

async function quickOpen2() {
  await cryptoWaitReady();
  
  const relayApi = await ApiPromise.create({ 
    provider: new WsProvider('ws://localhost:9944') 
  });
  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');

  console.log('Opening channel 1000 -> 2000 via relay chain sudo...\n');

  // Force open directly on relay chain
  await relayApi.tx.sudo.sudo(
    relayApi.tx.hrmp.forceOpenHrmpChannel(1000, 2000, 8, 8192)
  ).signAndSend(alice, ({ status }) => {
    console.log(`Status: ${status.type}`);
    if (status.isFinalized) {
      console.log('✅ Channel force-opened!');
      console.log('Run: node debug.js to verify');
      process.exit(0);
    }
  });
}

quickOpen2().catch(console.error);

