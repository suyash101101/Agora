const { ApiPromise, WsProvider } = require('@polkadot/api');

async function checkChannelRequest() {
  const relayApi = await ApiPromise.create({ 
    provider: new WsProvider('ws://localhost:9944') 
  });

  console.log('🔍 Checking HRMP status on relay chain...\n');
  
  // Check specific channels
  console.log('📋 Channel Status:');
  const channel1000to2000 = await relayApi.query.hrmp.hrmpChannels([1000, 2000]);
  const channel2000to1000 = await relayApi.query.hrmp.hrmpChannels([2000, 1000]);
  
  console.log(`  1000 -> 2000: ${channel1000to2000.isSome ? '✅ OPEN' : '❌ CLOSED'}`);
  if (channel1000to2000.isSome) {
    console.log(`    Details:`, channel1000to2000.toHuman());
  }
  
  console.log(`  2000 -> 1000: ${channel2000to1000.isSome ? '✅ OPEN' : '❌ CLOSED'}`);
  if (channel2000to1000.isSome) {
    console.log(`    Details:`, channel2000to1000.toHuman());
  }
  
  // Check pending requests
  console.log('\n📤 Pending Open Requests:');
  const request1000to2000 = await relayApi.query.hrmp.hrmpOpenChannelRequests([1000, 2000]);
  const request2000to1000 = await relayApi.query.hrmp.hrmpOpenChannelRequests([2000, 1000]);
  
  console.log(`  1000 -> 2000: ${request1000to2000.isSome ? '⏳ PENDING' : '❌ NONE'}`);
  if (request1000to2000.isSome) {
    console.log(`    Details:`, request1000to2000.toHuman());
  }
  
  console.log(`  2000 -> 1000: ${request2000to1000.isSome ? '⏳ PENDING' : '❌ NONE'}`);
  if (request2000to1000.isSome) {
    console.log(`    Details:`, request2000to1000.toHuman());
  }
  
  // Check all open channels
  console.log('\n📊 All HRMP Channels:');
  const allChannels = await relayApi.query.hrmp.hrmpChannels.entries();
  if (allChannels.length === 0) {
    console.log('  No channels found');
  } else {
    allChannels.forEach(([key, value]) => {
      const keyData = key.toHuman();
      console.log(`  ${keyData[0].sender} -> ${keyData[0].recipient}`);
    });
  }
  
  // Check all pending requests
  console.log('\n📨 All Pending Requests:');
  const allRequests = await relayApi.query.hrmp.hrmpOpenChannelRequests.entries();
  if (allRequests.length === 0) {
    console.log('  No pending requests');
  } else {
    allRequests.forEach(([key, value]) => {
      console.log('  Key:', key.toHuman());
      console.log('  Value:', value.toHuman());
    });
  }

  console.log('\n✅ Check complete!');
  await relayApi.disconnect();
  process.exit(0);
}

checkChannelRequest().catch(console.error);
