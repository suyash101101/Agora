import React, { useState } from 'react';
import { useApiContext } from '../../context/ApiContext';
import { useAccountContext } from '../../context/AccountContext';
import { MIN_JOB_BOUNTY, JobType, JOB_TYPE_LABELS } from '../../utils/constants';
import { parseBalance, formatBalance } from '../../utils/formatters';
import { validateJobInput, validateBounty } from '../../utils/helpers';
import { stringToBytes } from '../../utils/formatters';
import { signAndSend } from '../../utils/signer';
import { Send, Loader } from 'lucide-react';

export function SubmitJobForm() {
  const { api } = useApiContext();
  const { account, getSigner, balance } = useAccountContext();
  const [jobType, setJobType] = useState<JobType>(JobType.ApiRequest);
  const [inputData, setInputData] = useState('');
  const [bounty, setBounty] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!api || !account) {
      setError('Please connect your wallet first');
      return;
    }

    setError(null);

    // Validate input
    const inputValidation = validateJobInput(inputData);
    if (!inputValidation.valid) {
      setError(inputValidation.error || 'Invalid input');
      return;
    }

    const bountyBN = parseBalance(bounty);
    const bountyValidation = validateBounty(BigInt(bountyBN.toString()), MIN_JOB_BOUNTY);
    if (!bountyValidation.valid) {
      setError(bountyValidation.error || 'Invalid bounty');
      return;
    }

    try {
      setIsSubmitting(true);
      const signer = await getSigner(account.address);
      
      if (!signer) {
        setError('No signer available. Please use a wallet extension or test account for signing.');
        setIsSubmitting(false);
        return;
      }

      // Convert string to bytes - API expects Vec<u8>
      // Pass as array - API will handle SCALE encoding automatically
      const inputBytes = stringToBytes(inputData);
      
      // Create the transaction - pass array directly, API handles encoding
      const tx = api.tx.agora.submitJob(jobType, Array.from(inputBytes), bountyBN.toString());
      
      // Use helper function to handle both keypair and extension signers
      await signAndSend(tx, signer, account.address, ({ status, events }) => {
        console.log('Transaction status:', status.type);
        console.log('All events:', events.map(e => ({ 
          section: e.event.section, 
          method: e.event.method,
          data: e.event.data.toString()
        })));
        
        if (status.isInBlock) {
          const blockHash = status.asInBlock.toString();
          console.log('📦 Transaction in block:', blockHash);
          
          // Check if transaction was successful
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('❌ Transaction failed:', failed);
            const errorData = failed.event.data;
            console.error('Error data:', errorData);
            
            // Extract error details
            let errorMessage = 'Transaction failed';
            try {
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
            
            setError(errorMessage);
            setIsSubmitting(false);
            return;
          }
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
            setIsSubmitting(false);
            setError('Transaction failed on-chain. Check console for details.');
            return;
          }
          
          // Check for success event
          const success = events.find(e => e.event.method === 'JobSubmitted');
          if (success) {
            const jobData = success.event.data.toHuman();
            console.log('✅ JobSubmitted event confirmed:', jobData);
            setInputData('');
            setBounty('');
            setIsSubmitting(false);
            alert(`Job submitted successfully!\n\nJob ID: ${(jobData as any)?.jobId}\nBlock: ${blockHash}\n\nCheck Polkadot.js Apps to verify.`);
          } else {
            console.warn('⚠️ No JobSubmitted event found, but transaction finalized');
            setIsSubmitting(false);
            alert(`Transaction finalized!\n\nBlock: ${blockHash}\n\nCheck Polkadot.js Apps to verify.`);
          }
        }
      });
    } catch (error) {
      console.error('Failed to submit job:', error);
      setError(error instanceof Error ? error.message : 'Failed to submit job');
      setIsSubmitting(false);
    }
  };

  if (!account) {
    return (
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <p className="text-gray-500">Please connect your wallet to submit a job</p>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6">
      <h2 className="text-xl font-semibold mb-4">Submit New Job</h2>
      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Job Type
          </label>
          <select
            value={jobType}
            onChange={(e) => setJobType(parseInt(e.target.value) as JobType)}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
          >
            {Object.entries(JOB_TYPE_LABELS).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Input Data
          </label>
          <textarea
            value={inputData}
            onChange={(e) => setInputData(e.target.value)}
            placeholder="Enter job input data..."
            rows={4}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
          <p className="text-xs text-gray-500 mt-1">
            Maximum 1024 bytes
          </p>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 mb-2">
            Bounty (UNIT)
          </label>
          <input
            type="text"
            value={bounty}
            onChange={(e) => setBounty(e.target.value)}
            placeholder={`Minimum: ${formatBalance(MIN_JOB_BOUNTY)} UNIT`}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
          <p className="text-xs text-gray-500 mt-1">
            Minimum: {formatBalance(MIN_JOB_BOUNTY)} UNIT ({MIN_JOB_BOUNTY.toString()} raw)
          </p>
          {bounty && (
            <p className="text-xs text-blue-600 mt-1">
              You're entering: {formatBalance(parseBalance(bounty))} UNIT
            </p>
          )}
          {account && (
            <p className="text-xs text-gray-500 mt-1">
              Your balance: {balance} UNIT
            </p>
          )}
        </div>

        {error && (
          <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">
            {error}
          </div>
        )}

        <button
          type="submit"
          disabled={isSubmitting || !inputData.trim() || !bounty.trim()}
          className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isSubmitting ? (
            <>
              <Loader className="w-5 h-5 animate-spin" />
              <span>Submitting...</span>
            </>
          ) : (
            <>
              <Send className="w-5 h-5" />
              <span>Submit Job</span>
            </>
          )}
        </button>
      </form>
    </div>
  );
}

