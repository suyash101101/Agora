import React, { useState } from 'react';
import { useApiContext } from '../../context/ApiContext';
import { useAccountContext } from '../../context/AccountContext';
import { useWorkers } from '../../hooks/useWorkers';
import { MIN_WORKER_STAKE } from '../../utils/constants';
import { formatBalance, formatAddress, parseBalance } from '../../utils/formatters';
import { validateStake } from '../../utils/helpers';
import { signAndSend } from '../../utils/signer';
import { UserPlus, UserMinus, Coins, Award, Loader } from 'lucide-react';

export function WorkerDashboard() {
  const { api } = useApiContext();
  const { account, getSigner, balance } = useAccountContext();
  const { workers, loadWorker } = useWorkers(api);
  const [stake, setStake] = useState('');
  const [isRegistering, setIsRegistering] = useState(false);
  const [isUnregistering, setIsUnregistering] = useState(false);

  const worker = account ? workers.get(account.address) : null;

  React.useEffect(() => {
    if (account && api) {
      loadWorker(account.address);
    }
  }, [account, api, loadWorker]);

  const handleRegister = async () => {
    if (!api || !account) return;

    const stakeBN = parseBalance(stake);
    const validation = validateStake(BigInt(stakeBN.toString()), MIN_WORKER_STAKE);
    if (!validation.valid) {
      alert(validation.error);
      return;
    }

    try {
      setIsRegistering(true);
      const signer = await getSigner(account.address);
      if (!signer) {
        alert('No signer available');
        setIsRegistering(false);
        return;
      }
      
      const tx = api.tx.agora.registerWorker(stakeBN.toString());
      console.log('Registering worker with stake:', stakeBN.toString());
      console.log('Transaction:', tx.toHex());
      
      await signAndSend(tx, signer, account.address, ({ status, events }) => {
        console.log('Transaction status:', status.type);
        console.log('All events:', events.map(e => ({ 
          section: e.event.section, 
          method: e.event.method,
          data: e.event.data.toString()
        })));
        
        if (status.isInBlock) {
          console.log('Transaction in block:', status.asInBlock.toString());
          
          // Check for errors - ExtrinsicFailed contains the error
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('Transaction failed:', failed);
            console.error('Failed event data:', failed.event.data);
            
            // Extract error details
            let errorMessage = 'Transaction failed';
            try {
              const errorData = failed.event.data;
              const errorInfo = errorData.toHuman() as any;
              console.error('Error info (toHuman):', errorInfo);
              console.error('Error info (toJSON):', errorData.toJSON());
              
              // ExtrinsicFailed data structure: {dispatchError: {...}, dispatchInfo: {...}}
              const dispatchError = errorInfo.dispatchError || (Array.isArray(errorInfo) ? errorInfo[0] : errorInfo);
              console.error('Dispatch error:', dispatchError);
              console.error('Dispatch error (full):', JSON.stringify(dispatchError, null, 2));
              
              // Check different error types
              if (dispatchError?.Module) {
                const moduleError = dispatchError.Module;
                console.error('Module error details:', moduleError);
                const errorIndexHex = moduleError.error;
                const moduleIndex = parseInt(moduleError.index);
                
                // Parse error index from hex (e.g., "0x0e000000" -> 14)
                // Error is encoded as: (error_index << 24) | module_index
                // Or just extract the first byte
                let errorIndex: number;
                if (typeof errorIndexHex === 'string' && errorIndexHex.startsWith('0x')) {
                  // Parse hex: "0x0e000000" -> get first byte -> 0x0e -> 14
                  errorIndex = parseInt(errorIndexHex.slice(2, 4), 16);
                } else {
                  errorIndex = parseInt(errorIndexHex.toString());
                }
                
                console.error(`Module Index: ${moduleIndex}, Error Index: ${errorIndex} (from ${errorIndexHex})`);
                
                // Map error index to error name (based on agora pallet Error enum)
                const errorNames: Record<number, string> = {
                  0: 'JobNotFound',
                  1: 'WorkerNotRegistered',
                  2: 'WorkerAlreadyRegistered',
                  3: 'InsufficientStake',
                  4: 'InsufficientBounty',
                  5: 'InvalidJobPhase',
                  6: 'CommitMismatch',
                  7: 'AlreadyCommitted',
                  8: 'NotCommitted',
                  9: 'CommitDeadlinePassed',
                  10: 'RevealDeadlinePassed',
                  11: 'JobAlreadyFinalized',
                  12: 'InsufficientReveals',
                  13: 'InputDataTooLarge',
                  14: 'InsufficientBalance',
                  15: 'WorkerUnbonding',
                  16: 'TooManyConcurrentJobs',
                  17: 'SaltVerificationFailed',
                  18: 'AlreadyRevealed',
                  19: 'JobCancelled',
                  20: 'UnbondingPeriodNotCompleted',
                  21: 'XcmSendFailed',
                  22: 'Overflow',
                };
                
                const errorName = errorNames[errorIndex] || `Unknown(${errorIndex})`;
                errorMessage = `Error: agora.${errorName}`;
                
                // Try to decode using API as fallback
                if (api) {
                  try {
                    const errorMeta = api.registry.findMetaError({ 
                      index: moduleIndex, 
                      error: errorIndex 
                    });
                    
                    if (errorMeta) {
                      errorMessage = `Error: ${errorMeta.section}.${errorMeta.name}`;
                      console.error('Decoded error:', errorMeta);
                    }
                  } catch (e) {
                    console.error('Error decoding:', e);
                    // Use the mapped error name we already have
                  }
                }
              } else if (dispatchError?.BadOrigin) {
                errorMessage = 'Bad origin: Unauthorized';
              } else if (dispatchError?.CannotLookup) {
                errorMessage = 'Cannot lookup: Account not found';
              } else if (dispatchError?.Other) {
                errorMessage = `Other error: ${dispatchError.Other}`;
              } else {
                errorMessage = `Error: ${JSON.stringify(dispatchError)}`;
              }
            } catch (e) {
              console.error('Error parsing failed event:', e);
              errorMessage = `Transaction failed: ${failed.event.data.toString()}`;
            }
            
            setIsRegistering(false);
            alert(`Transaction failed: ${errorMessage}\n\nCheck console for full error details.`);
            return;
          }
          
          // Check for success event
          const workerRegistered = events.find(e => 
            e.event.section === 'agora' && e.event.method === 'WorkerRegistered'
          );
          if (workerRegistered) {
            console.log('Worker registered event found:', workerRegistered.event.data.toHuman());
          }
          
          // Reload worker data
          loadWorker(account.address);
        }
        
        if (status.isFinalized) {
          const blockHash = status.asFinalized.toString();
          console.log('✅ Transaction finalized in block:', blockHash);
          console.log('📊 Final events:', events.map(e => ({
            section: e.event.section,
            method: e.event.method,
            data: e.event.data.toHuman(),
          })));
          
          // Verify transaction was actually successful
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('❌ Transaction failed even though finalized:', failed);
            setIsRegistering(false);
            alert('Transaction failed on-chain. Check console for details.');
            return;
          }
          
          const workerRegistered = events.find(e => 
            e.event.section === 'agora' && e.event.method === 'WorkerRegistered'
          );
          if (!workerRegistered) {
            console.warn('⚠️ No WorkerRegistered event found, but transaction finalized');
          } else {
            console.log('✅ WorkerRegistered event confirmed:', workerRegistered.event.data.toHuman());
          }
          
          setStake('');
          setIsRegistering(false);
          alert(`Worker registered successfully!\n\nBlock: ${blockHash}\n\nCheck Polkadot.js Apps to verify.`);
        }
        
        if (status.isError || status.isInvalid || status.isDropped) {
          console.error('Transaction error:', status.type);
          setIsRegistering(false);
          alert(`Transaction failed: ${status.type}`);
        }
      });
    } catch (error) {
      console.error('Failed to register worker:', error);
      setIsRegistering(false);
      alert(`Failed to register: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const handleUnregister = async () => {
    if (!api || !account) return;

    if (!confirm('Are you sure you want to unregister as a worker?')) {
      return;
    }

    try {
      setIsUnregistering(true);
      const signer = await getSigner(account.address);
      if (!signer) {
        alert('No signer available');
        setIsUnregistering(false);
        return;
      }
      
      const tx = api.tx.agora.unregisterWorker();
      console.log('Unregistering worker');
      console.log('Transaction:', tx.toHex());
      
      await signAndSend(tx, signer, account.address, ({ status, events }) => {
        console.log('Transaction status:', status.type);
        console.log('All events:', events.map(e => ({ 
          section: e.event.section, 
          method: e.event.method,
          data: e.event.data.toString()
        })));
        
        if (status.isInBlock) {
          console.log('Transaction in block:', status.asInBlock.toString());
          
          // Check for errors
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('Transaction failed:', failed);
            setIsUnregistering(false);
            alert('Transaction failed. Check console for details.');
            return;
          }
          
          // Reload worker data
          loadWorker(account.address);
        }
        
        if (status.isFinalized) {
          console.log('Transaction finalized:', status.asFinalized.toString());
          setIsUnregistering(false);
          alert('Worker unregistered successfully!');
        }
        
        if (status.isError || status.isInvalid || status.isDropped) {
          console.error('Transaction error:', status.type);
          setIsUnregistering(false);
          alert(`Transaction failed: ${status.type}`);
        }
      });
    } catch (error) {
      console.error('Failed to unregister worker:', error);
      setIsUnregistering(false);
      alert(`Failed to unregister: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  if (!account) {
    return (
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <p className="text-gray-500">Please connect your wallet to view worker dashboard</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {worker ? (
        <div className="bg-white rounded-lg border border-gray-200 p-6">
          <h2 className="text-xl font-semibold mb-4">Worker Status</h2>
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <UserPlus className="w-5 h-5 text-green-600" />
              <span className="font-medium">Registered Worker</span>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <p className="text-sm text-gray-500">Stake</p>
                <p className="font-medium flex items-center gap-2">
                  <Coins className="w-4 h-4" />
                  {formatBalance(worker.stake.toString())} UNIT
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500">Reputation</p>
                <p className="font-medium flex items-center gap-2">
                  <Award className="w-4 h-4" />
                  {Number(worker.reputation)}/1000
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500">Status</p>
                <p className="font-medium">
                  {worker.isActive ? 'Active' : 'Inactive'}
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500">Registered At</p>
                <p className="font-medium">Block {worker.registeredAt.toString()}</p>
              </div>
            </div>
            <button
              onClick={handleUnregister}
              disabled={isUnregistering}
              className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isUnregistering ? (
                <>
                  <Loader className="w-5 h-5 animate-spin" />
                  <span>Unregistering...</span>
                </>
              ) : (
                <>
                  <UserMinus className="w-5 h-5" />
                  <span>Unregister Worker</span>
                </>
              )}
            </button>
          </div>
        </div>
      ) : (
               <div className="bg-white rounded-lg border border-gray-200 p-6">
                 <h2 className="text-xl font-semibold mb-4">Register as Worker</h2>
                 <div className="space-y-4">
                   <div>
                     <label className="block text-sm font-medium text-gray-700 mb-2">
                       Stake Amount (UNIT)
                     </label>
                     <input
                       type="text"
                       value={stake}
                       onChange={(e) => setStake(e.target.value)}
                       placeholder={`Minimum: ${formatBalance(MIN_WORKER_STAKE)} UNIT`}
                       className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                     />
                     <p className="text-xs text-gray-500 mt-1">
                       Minimum: {formatBalance(MIN_WORKER_STAKE)} UNIT ({MIN_WORKER_STAKE.toString()} raw)
                     </p>
                     {stake && (
                       <p className="text-xs text-blue-600 mt-1">
                         You're entering: {formatBalance(parseBalance(stake))} UNIT
                       </p>
                     )}
                     {account && (
                       <p className="text-xs text-gray-500 mt-1">
                         Your balance: {balance} UNIT
                       </p>
                     )}
                   </div>
            <button
              onClick={handleRegister}
              disabled={isRegistering || !stake.trim()}
              className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isRegistering ? (
                <>
                  <Loader className="w-5 h-5 animate-spin" />
                  <span>Registering...</span>
                </>
              ) : (
                <>
                  <UserPlus className="w-5 h-5" />
                  <span>Register Worker</span>
                </>
              )}
            </button>
          </div>
        </div>
      )}

      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <h2 className="text-xl font-semibold mb-4">All Workers</h2>
        <div className="space-y-3">
          {Array.from(workers.entries()).map(([address, workerInfo]) => (
            <div key={address} className="bg-gray-50 p-4 rounded-lg">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">{formatAddress(address)}</p>
                  <p className="text-sm text-gray-500">
                    Reputation: {Number(workerInfo.reputation)}/1000
                  </p>
                </div>
                <div className="text-right">
                  <p className="font-medium">{formatBalance(workerInfo.stake.toString())} UNIT</p>
                  <p className="text-xs text-gray-500">
                    {workerInfo.isActive ? 'Active' : 'Inactive'}
                  </p>
                  <p className="text-xs text-gray-500">
                    Registered at: Block {workerInfo.registeredAt.toString()}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

