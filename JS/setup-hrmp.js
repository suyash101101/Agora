const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { cryptoWaitReady, encodeAddress } = require('@polkadot/util-crypto');
const { u8aConcat } = require('@polkadot/util');

function paraIdToSovereignAccount(paraId, ss58Format = 42) {
  const prefix = new Uint8Array([0x70, 0x61, 0x72, 0x61]);
  const idBytes = new Uint8Array(4);
  const view = new DataView(idBytes.buffer);
  view.setUint32(0, paraId, true);
  const padding = new Uint8Array(24).fill(0);
  const accountId = u8aConcat(prefix, idBytes, padding);
  return encodeAddress(accountId, ss58Format);
}

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
        if (unsub) unsub();
        reject(new Error(`Transaction failed with status: ${result.status.type}`));
      }
    }).then((unsubscribe) => {
      unsub = unsubscribe;
    }).catch(reject);
  });
}

async function checkChannelExists(relayApi, sender, recipient) {
  const channel = await relayApi.query.hrmp.hrmpChannels([sender, recipient]);
  return channel.isSome;
}

async function waitForChannelRequest(relayApi, sender, recipient, maxAttempts = 20) {
  console.log(`    Waiting for channel request ${sender} -> ${recipient} on relay chain...`);
  for (let i = 0; i < maxAttempts; i++) {
    await sleep(6000);
    const request = await relayApi.query.hrmp.hrmpOpenChannelRequests([sender, recipient]);
    if (request.isSome) {
      console.log(`    ✅ Channel request found!\n`);
      return true;
    }
    console.log(`    Checking... attempt ${i + 1}/${maxAttempts}`);
  }
  throw new Error(`Channel request ${sender} -> ${recipient} not found after ${maxAttempts} attempts`);
}

async function openChannel(paraApi, relayApi, sender, recipient, alice) {
  console.log(`\n📤 Opening channel ${sender} -> ${recipient}...`);
  
  // Check if already exists
  const exists = await checkChannelExists(relayApi, sender, recipient);
  if (exists) {
    console.log(`✅ Channel ${sender} -> ${recipient} already exists. Skipping.\n`);
    return;
  }
  
  // Encode HRMP init call
  const encodedCall = `0x3c00${recipient.toString(16).padStart(8, '0').match(/../g).reverse().join('')}0800000000001000`;
  
  const dest = { V4: { parents: 1, interior: { Here: null } } };
  const message = {
    V4: [
      {
        WithdrawAsset: [{
          id: { parents: 0, interior: { Here: null } },
          fun: { Fungible: 1_000_000_000_000n }
        }]
      },
      {
        BuyExecution: {
          fees: {
            id: { parents: 0, interior: { Here: null } },
            fun: { Fungible: 1_000_000_000_000n }
          },
          weightLimit: { Unlimited: null }
        }
      },
      {
        Transact: {
          originKind: 'Native',
          requireWeightAtMost: { refTime: 1_000_000_000n, proofSize: 200_000n },
          call: { encoded: encodedCall }
        }
      },
      { RefundSurplus: null },
      {
        DepositAsset: {
          assets: { Wild: { All: null } },
          beneficiary: {
            parents: 0,
            interior: { X1: [{ Parachain: sender }] }
          }
        }
      }
    ]
  };

  await sendAndWait(
    paraApi.tx.sudo.sudo(paraApi.tx.polkadotXcm.send(dest, message)),
    alice
  );
  console.log(`✅ Opening request sent\n`);
  
  // Wait for request to appear on relay chain
  await waitForChannelRequest(relayApi, sender, recipient);
}

async function acceptChannel(paraApi, relayApi, sender, recipient, alice) {
  console.log(`\n📥 Accepting channel ${sender} -> ${recipient}...`);
  
  // Check if channel already exists (meaning it was already accepted)
  const exists = await checkChannelExists(relayApi, sender, recipient);
  if (exists) {
    console.log(`✅ Channel ${sender} -> ${recipient} already accepted. Skipping.\n`);
    return;
  }
  
  // Encode HRMP accept call
  const encodedCall = `0x3c01${sender.toString(16).padStart(8, '0').match(/../g).reverse().join('')}`;
  
  const dest = { V4: { parents: 1, interior: { Here: null } } };
  const message = {
    V4: [
      {
        WithdrawAsset: [{
          id: { parents: 0, interior: { Here: null } },
          fun: { Fungible: 1_000_000_000_000n }
        }]
      },
      {
        BuyExecution: {
          fees: {
            id: { parents: 0, interior: { Here: null } },
            fun: { Fungible: 1_000_000_000_000n }
          },
          weightLimit: { Unlimited: null }
        }
      },
      {
        Transact: {
          originKind: 'Native',
          requireWeightAtMost: { refTime: 1_000_000_000n, proofSize: 200_000n },
          call: { encoded: encodedCall }
        }
      },
      { RefundSurplus: null },
      {
        DepositAsset: {
          assets: { Wild: { All: null } },
          beneficiary: {
            parents: 0,
            interior: { X1: [{ Parachain: recipient }] }
          }
        }
      }
    ]
  };

  await sendAndWait(
    paraApi.tx.sudo.sudo(paraApi.tx.polkadotXcm.send(dest, message)),
    alice
  );
  console.log(`✅ Acceptance sent\n`);
  await sleep(12000); // Wait for channel to be established
}

async function setupHRMPAndFunding() {
  await cryptoWaitReady();
  
  console.log('🚀 Connecting to networks...\n');
  
  const relayApi = await ApiPromise.create({ 
    provider: new WsProvider('ws://localhost:9944') 
  });
  const para1000Api = await ApiPromise.create({ 
    provider: new WsProvider('ws://localhost:9946') 
  });
  const para2000Api = await ApiPromise.create({ 
    provider: new WsProvider('ws://localhost:9947') 
  });

  const keyring = new Keyring({ type: 'sr25519' });
  const alice = keyring.addFromUri('//Alice');

  const relayChainSS58 = relayApi.registry.chainSS58 || 42;
  const para1000SS58 = para1000Api.registry.chainSS58 || 42;
  const para2000SS58 = para2000Api.registry.chainSS58 || 42;

  const para1000SovereignOnRelay = paraIdToSovereignAccount(1000, relayChainSS58);
  const para2000SovereignOnRelay = paraIdToSovereignAccount(2000, relayChainSS58);
  const para1000SovereignOnPara2000 = paraIdToSovereignAccount(1000, para2000SS58);
  const para2000SovereignOnPara1000 = paraIdToSovereignAccount(2000, para1000SS58);

  console.log('✅ Connected to all chains');
  console.log(`\n📋 Sovereign Accounts:`);
  console.log(`Para 1000 on Relay: ${para1000SovereignOnRelay}`);
  console.log(`Para 2000 on Relay: ${para2000SovereignOnRelay}`);
  console.log(`Para 1000 on Para 2000: ${para1000SovereignOnPara2000}`);
  console.log(`Para 2000 on Para 1000: ${para2000SovereignOnPara1000}\n`);

  // Check existing channels first
  console.log('🔍 Checking existing HRMP channels...\n');
  const channel1000to2000 = await checkChannelExists(relayApi, 1000, 2000);
  const channel2000to1000 = await checkChannelExists(relayApi, 2000, 1000);
  
  console.log(`Channel 1000 -> 2000: ${channel1000to2000 ? '✅ EXISTS' : '❌ NOT FOUND'}`);
  console.log(`Channel 2000 -> 1000: ${channel2000to1000 ? '✅ EXISTS' : '❌ NOT FOUND'}\n`);

  // ========== PART 1: Fund Relay Chain Sovereign Accounts ==========
  console.log('💰 PART 1: Funding sovereign accounts on relay chain...\n');

  console.log('Funding para 1000 sovereign on relay...');
  await sendAndWait(
    relayApi.tx.balances.transferKeepAlive(para1000SovereignOnRelay, 10_000_000_000_000n),
    alice
  );

  console.log('\nFunding para 2000 sovereign on relay...');
  await sendAndWait(
    relayApi.tx.balances.transferKeepAlive(para2000SovereignOnRelay, 10_000_000_000_000n),
    alice
  );

  console.log('\n✅ Relay chain sovereign accounts funded\n');
  await sleep(3000);

  // ========== PART 2: Fund Cross-Parachain Sovereign Accounts ==========
  console.log('💰 PART 2: Funding cross-parachain sovereign accounts...\n');

  console.log('Funding para 1000 sovereign on para 2000...');
  await sendAndWait(
    para2000Api.tx.balances.transferKeepAlive(para1000SovereignOnPara2000, 100_000_000_000_000n),
    alice
  );

  console.log('\nFunding para 2000 sovereign on para 1000...');
  await sendAndWait(
    para1000Api.tx.balances.transferKeepAlive(para2000SovereignOnPara1000, 100_000_000_000_000n),
    alice
  );

  console.log('\n✅ Cross-parachain sovereign accounts funded\n');
  await sleep(3000);

  // ========== PART 3: Open HRMP Channels (Only if Needed) ==========
  console.log('🔗 PART 3: Setting up HRMP channels...\n');

  try {
    // Open and accept channel 1000 -> 2000
    await openChannel(para1000Api, relayApi, 1000, 2000, alice);
    await acceptChannel(para2000Api, relayApi, 1000, 2000, alice);

    // Open and accept reverse channel 2000 -> 1000
    await openChannel(para2000Api, relayApi, 2000, 1000, alice);
    await acceptChannel(para1000Api, relayApi, 2000, 1000, alice);
  } catch (error) {
    console.error('⚠️  HRMP setup error:', error.message);
    console.log('Continuing to verification...\n');
  }

  // ========== Verify Final State ==========
  console.log('🔍 Verifying final state...\n');
  await sleep(6000);
  
  const finalChannel1 = await checkChannelExists(relayApi, 1000, 2000);
  const finalChannel2 = await checkChannelExists(relayApi, 2000, 1000);
  
  console.log('📋 Final HRMP Channel Status:');
  console.log(`  1000 -> 2000: ${finalChannel1 ? '✅ OPEN' : '❌ CLOSED'}`);
  console.log(`  2000 -> 1000: ${finalChannel2 ? '✅ OPEN' : '❌ CLOSED'}\n`);

  if (finalChannel1 && finalChannel2) {
    console.log('🎉 Setup complete! All channels are open and accounts are funded.');
    console.log('💡 You can now send XCM job requests between parachains!\n');
  } else {
    console.log('⚠️  Setup completed but some channels are not open.');
    console.log('💡 Try running the script again or open channels manually via Polkadot.js Apps.\n');
  }
  
  await relayApi.disconnect();
  await para1000Api.disconnect();
  await para2000Api.disconnect();
  process.exit(0);
}

setupHRMPAndFunding().catch((error) => {
  console.error('❌ Error:', error);
  process.exit(1);
});
