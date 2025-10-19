#!/usr/bin/env node

const { ApiPromise, WsProvider } = require('@polkadot/api');

async function testOCW() {
    console.log('🔧 Testing OCW Infrastructure Setup...');
    
    // Connect to parachain
    const provider = new WsProvider('ws://localhost:9988');
    const api = await ApiPromise.create({ provider });
    
    console.log('✅ Connected to parachain');
    
    // Get Alice account (well-known test account)
    const alice = api.registry.createType('AccountId', '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY');
    
    console.log('📋 Submitting test job...');
    
    // Submit a job
    const jobType = api.registry.createType('JobType', 'Computation');
    const inputData = api.registry.createType('BoundedVec<u8, ConstU32<2048>>', 'Hello World');
    const bounty = api.registry.createType('Balance', 1000000000000); // 1 DOT
    
    const tx = api.tx.agora.submitJob(jobType, inputData, bounty);
    
    try {
        const hash = await tx.signAndSend(alice);
        console.log('✅ Job submitted successfully:', hash.toHex());
        
        // Wait a few blocks for OCW to process
        console.log('⏳ Waiting for OCW to process job...');
        await new Promise(resolve => setTimeout(resolve, 10000));
        
        // Check if job was created
        const jobs = await api.query.agora.jobs.entries();
        console.log('📊 Total jobs in storage:', jobs.length);
        
        if (jobs.length > 0) {
            console.log('🎉 Job created successfully!');
            console.log('Job details:', jobs[0][1].toHuman());
        }
        
    } catch (error) {
        console.error('❌ Error submitting job:', error.message);
    }
    
    await api.disconnect();
}

testOCW().catch(console.error);
