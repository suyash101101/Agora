const { ApiPromise, WsProvider } = require('@polkadot/api');
const { encodeAddress } = require('@polkadot/util-crypto');
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

async function checkBalances() {
  console.log('🔍 Checking sovereign account balances...\n');
  
  const relayApi = await ApiPromise.create({ 
    provider: new WsProvider('ws://localhost:9944') 
  });
  const para1000Api = await ApiPromise.create({ 
    provider: new WsProvider('ws://localhost:9946') 
  });
  const para2000Api = await ApiPromise.create({ 
    provider: new WsProvider('ws://localhost:9947') 
  });

  const relayChainSS58 = relayApi.registry.chainSS58 || 42;
  const para1000SS58 = para1000Api.registry.chainSS58 || 42;
  const para2000SS58 = para2000Api.registry.chainSS58 || 42;

  const para1000SovereignOnRelay = paraIdToSovereignAccount(1000, relayChainSS58);
  const para2000SovereignOnRelay = paraIdToSovereignAccount(2000, relayChainSS58);
  const para1000SovereignOnPara2000 = paraIdToSovereignAccount(1000, para2000SS58);
  const para2000SovereignOnPara1000 = paraIdToSovereignAccount(2000, para1000SS58);

  console.log('📋 Sovereign Accounts:');
  console.log(`Para 1000 on Relay: ${para1000SovereignOnRelay}`);
  console.log(`Para 2000 on Relay: ${para2000SovereignOnRelay}`);
  console.log(`Para 1000 on Para 2000: ${para1000SovereignOnPara2000}`);
  console.log(`Para 2000 on Para 1000: ${para2000SovereignOnPara1000}\n`);

  // Check balances on relay chain
  console.log('💰 Balances on Relay Chain:');
  const balance1000OnRelay = await relayApi.query.system.account(para1000SovereignOnRelay);
  const balance2000OnRelay = await relayApi.query.system.account(para2000SovereignOnRelay);
  
  console.log(`  Para 1000 sovereign: ${balance1000OnRelay.data.free.toHuman()}`);
  console.log(`  Para 2000 sovereign: ${balance2000OnRelay.data.free.toHuman()}\n`);

  // Check balances on para 1000
  console.log('💰 Balances on Para 1000:');
  const balance2000OnPara1000 = await para1000Api.query.system.account(para2000SovereignOnPara1000);
  console.log(`  Para 2000 sovereign: ${balance2000OnPara1000.data.free.toHuman()}\n`);

  // Check balances on para 2000
  console.log('💰 Balances on Para 2000:');
  const balance1000OnPara2000 = await para2000Api.query.system.account(para1000SovereignOnPara2000);
  console.log(`  Para 1000 sovereign: ${balance1000OnPara2000.data.free.toHuman()}\n`);

  // Check if balances are sufficient
  console.log('✅ Balance Status:');
  const minRequired = BigInt(1_000_000_000_000); // 1 token
  
  const relay1000Ok = balance1000OnRelay.data.free.toBigInt() >= minRequired;
  const relay2000Ok = balance2000OnRelay.data.free.toBigInt() >= minRequired;
  const para1000Ok = balance2000OnPara1000.data.free.toBigInt() >= minRequired;
  const para2000Ok = balance1000OnPara2000.data.free.toBigInt() >= minRequired;

  console.log(`  Para 1000 on Relay: ${relay1000Ok ? '✅ OK' : '❌ INSUFFICIENT'}`);
  console.log(`  Para 2000 on Relay: ${relay2000Ok ? '✅ OK' : '❌ INSUFFICIENT'}`);
  console.log(`  Para 2000 on Para 1000: ${para1000Ok ? '✅ OK' : '❌ INSUFFICIENT'}`);
  console.log(`  Para 1000 on Para 2000: ${para2000Ok ? '✅ OK' : '❌ INSUFFICIENT'}\n`);

  if (!para2000Ok) {
    console.log('⚠️  Para 1000 sovereign on Para 2000 needs funding!');
    console.log('   This is needed for 1000 -> 2000 XCM messages.');
  }
  if (!para1000Ok) {
    console.log('⚠️  Para 2000 sovereign on Para 1000 needs funding!');
    console.log('   This is needed for 2000 -> 1000 XCM messages.');
  }

  await relayApi.disconnect();
  await para1000Api.disconnect();
  await para2000Api.disconnect();
  process.exit(0);
}

checkBalances().catch(console.error);

