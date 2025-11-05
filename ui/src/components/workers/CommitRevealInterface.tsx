import React, { useState } from 'react';
import { useApiContext } from '../../context/ApiContext';
import { useAccountContext } from '../../context/AccountContext';
import { useJobs } from '../../hooks/useJobs';
import { generateSalt, generateCommitHash, generateCommitHashBytes, ensureSalt32Bytes, hashHexToHash } from '../../utils/helpers';
import { stringToBytes } from '../../utils/formatters';
import { formatBalance } from '../../utils/formatters';
import { JobStatus } from '../../utils/constants';
import { signAndSend } from '../../utils/signer';
import { Loader, Eye, EyeOff, Copy, Check } from 'lucide-react';

export function CommitRevealInterface() {
  const { api } = useApiContext();
  const { account, getSigner } = useAccountContext();
  const { jobs, commits, reveals, loadCommits, loadReveals } = useJobs(api);
  const [currentBlock, setCurrentBlock] = React.useState<number>(0);
  const [selectedJobId, setSelectedJobId] = useState<number | null>(null);
  const [result, setResult] = useState('');
  const [salt, setSalt] = useState<Uint8Array | null>(null);
  const [commitHash, setCommitHash] = useState<string | null>(null);
  const [isCommitting, setIsCommitting] = useState(false);
  const [isRevealing, setIsRevealing] = useState(false);
  const [copied, setCopied] = useState(false);

  React.useEffect(() => {
    if (api) {
      api.rpc.chain.subscribeNewHeads((header) => {
        setCurrentBlock(header.number.toNumber());
      });
    }
  }, [api]);

  const availableJobs = Array.from(jobs.entries()).filter(([jobId, job]) => {
    const jobCommits = commits.get(jobId) || [];
    const jobReveals = reveals.get(jobId) || [];
    const hasCommitted = jobCommits.some(c => c.worker.toString() === account?.address);
    const hasRevealed = jobReveals.some(r => r.worker.toString() === account?.address);
    
    if (job.status.toString() === JobStatus.Pending || job.status.toString() === JobStatus.CommitPhase) {
      return !hasCommitted && currentBlock <= job.commitDeadline.toNumber();
    }
    if (job.status.toString() === JobStatus.RevealPhase) {
      return hasCommitted && !hasRevealed && currentBlock > job.commitDeadline.toNumber() && currentBlock <= job.revealDeadline.toNumber();
    }
    return false;
  });

  const handleGenerateCommit = () => {
    if (!result.trim()) {
      alert('Please enter a result first');
      return;
    }
    const newSalt = generateSalt();
    const resultBytes = stringToBytes(result);
    const hash = generateCommitHash(newSalt, resultBytes);
    setSalt(newSalt);
    setCommitHash(hash);
  };

  const handleCopyHash = async () => {
    if (!commitHash) return;
    try {
      await navigator.clipboard.writeText(commitHash);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  };

  const handleCommit = async () => {
    if (!api || !account || !selectedJobId || !salt || !commitHash) return;

    try {
      setIsCommitting(true);
      
      // Ensure salt is exactly 32 bytes
      const salt32 = ensureSalt32Bytes(salt);
      
      // Convert salt to array format expected by API ([u8; 32])
      const saltArray = Array.from(salt32);
      
      // Convert commit hash hex string to proper format
      // The API expects a Hash type, which can be created from hex string
      const hashHex = hashHexToHash(commitHash);
      
      console.log('🔐 Committing result:', {
        jobId: selectedJobId,
        salt: saltArray,
        saltLength: saltArray.length,
        commitHash: hashHex,
        commitHashLength: hashHex.length,
      });

      const signer = await getSigner(account.address);
      if (!signer) {
        alert('No signer available');
        setIsCommitting(false);
        return;
      }
      
      // API expects: commitResult(jobId, salt: [u8; 32], result_hash: Hash)
      // salt should be passed as array of 32 bytes
      // hash should be passed as hex string (API will convert to Hash type)
      const tx = api.tx.agora.commitResult(selectedJobId, saltArray, hashHex);
      
      await signAndSend(tx, signer, account.address, ({ status, events }) => {
        console.log('📡 Commit transaction status:', status.type);
        
        if (status.isInBlock) {
          const blockHash = status.asInBlock.toString();
          console.log('📦 Commit transaction in block:', blockHash);
          
          // Check for errors
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('❌ Commit transaction failed:', failed);
            setIsCommitting(false);
            alert('Commit transaction failed. Check console for details.');
            return;
          }
          
          // Check for success event
          const committed = events.find(e => 
            e.event.section === 'agora' && e.event.method === 'ResultCommitted'
          );
          if (committed) {
            console.log('✅ ResultCommitted event found:', committed.event.data.toHuman());
          }
          
          loadCommits(selectedJobId);
        }
        
        if (status.isFinalized) {
          const blockHash = status.asFinalized.toString();
          console.log('✅ Commit transaction finalized in block:', blockHash);
          console.log('📊 Final events:', events.map(e => ({
            section: e.event.section,
            method: e.event.method,
            data: e.event.data.toHuman(),
          })));
          
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('❌ Commit transaction failed:', failed);
            setIsCommitting(false);
            alert('Commit transaction failed on-chain. Check console for details.');
            return;
          }
          
          setResult('');
          setSalt(null);
          setCommitHash(null);
          setIsCommitting(false);
          
          // Store the committed result in localStorage for later retrieval
          if (selectedJobId !== null) {
            const storageKey = `agora_commit_${selectedJobId}_${account.address}`;
            localStorage.setItem(storageKey, result);
            console.log('💾 Stored committed result in localStorage:', storageKey);
          }
          
          alert(`Commit submitted successfully!\n\nBlock: ${blockHash}\n\nCheck Polkadot.js Apps to verify.`);
        }
      });
    } catch (error) {
      console.error('Failed to commit:', error);
      setIsCommitting(false);
      alert(`Failed to commit: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const handleReveal = async () => {
    if (!api || !account || !selectedJobId || !result.trim()) return;

    const jobCommits = commits.get(selectedJobId);
    const myCommit = jobCommits?.find(c => c.worker.toString() === account.address);
    
    if (!myCommit) {
      alert('You must commit first before revealing');
      return;
    }

    try {
      setIsRevealing(true);
      
      const resultBytes = stringToBytes(result);
      
      // Verify the hash locally before submitting
      const { generateCommitHashBytes } = await import('../../utils/helpers');
      const { u8aConcat } = await import('@polkadot/util');
      
      // Get salt from commit (it's stored as Uint8Array in the commit)
      const commitSalt = myCommit.salt;
      let saltU8a: Uint8Array;
      
      if (commitSalt instanceof Uint8Array) {
        saltU8a = commitSalt;
      } else if (Array.isArray(commitSalt)) {
        saltU8a = Uint8Array.from(commitSalt);
      } else {
        // Try to convert from hex or other formats
        saltU8a = new Uint8Array(32);
        console.warn('Unexpected salt format:', commitSalt);
      }
      
      // Calculate hash locally to verify
      const calculatedHash = generateCommitHashBytes(saltU8a, resultBytes);
      const committedHash = myCommit.resultHash;
      
      console.log('🔍 Verifying reveal:', {
        jobId: selectedJobId,
        resultLength: resultBytes.length,
        result: Array.from(resultBytes),
        saltLength: saltU8a.length,
        salt: Array.from(saltU8a),
        calculatedHash: Array.from(calculatedHash),
        committedHash: committedHash.toString(),
      });
      
      // Note: We can't directly compare hashes here because committedHash is a Hash type
      // The runtime will verify it, but we can warn the user if format looks wrong
      
      const signer = await getSigner(account.address);
      if (!signer) {
        alert('No signer available');
        setIsRevealing(false);
        return;
      }
      
      // API expects: revealResult(jobId, result: Vec<u8>)
      const tx = api.tx.agora.revealResult(selectedJobId, Array.from(resultBytes));
      
      await signAndSend(tx, signer, account.address, ({ status, events }) => {
        console.log('📡 Reveal transaction status:', status.type);
        
        if (status.isInBlock) {
          const blockHash = status.asInBlock.toString();
          console.log('📦 Reveal transaction in block:', blockHash);
          
          // Check for errors
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('❌ Reveal transaction failed:', failed);
            
            // Extract detailed error information
            let errorMessage = 'Reveal transaction failed';
            try {
              const errorData = failed.event.data;
              const errorInfo = errorData.toHuman() as any;
              console.error('Error info (toHuman):', errorInfo);
              console.error('Error info (toJSON):', errorData.toJSON());
              
              const dispatchError = errorInfo.dispatchError || (Array.isArray(errorInfo) ? errorInfo[0] : errorInfo);
              
              if (dispatchError?.Module) {
                const moduleError = dispatchError.Module;
                const errorIndexHex = moduleError.error;
                const moduleIndex = parseInt(moduleError.index);
                
                let errorIndex: number;
                if (typeof errorIndexHex === 'string' && errorIndexHex.startsWith('0x')) {
                  errorIndex = parseInt(errorIndexHex.slice(2, 4), 16);
                } else {
                  errorIndex = parseInt(errorIndexHex.toString());
                }
                
                console.error(`Module Index: ${moduleIndex}, Error Index: ${errorIndex}`);
                
                // Map error index to error name
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
                  17: 'SaltVerificationFailed', // This is the likely error!
                  18: 'AlreadyRevealed',
                  19: 'JobCancelled',
                  20: 'UnbondingPeriodNotCompleted',
                  21: 'XcmSendFailed',
                  22: 'Overflow',
                };
                
                const errorName = errorNames[errorIndex] || `Unknown(${errorIndex})`;
                
                if (errorName === 'SaltVerificationFailed') {
                  errorMessage = 'Salt verification failed! The result you entered does not match the hash you committed. Make sure you enter the EXACT same result text you used when committing.';
                } else {
                  errorMessage = `Error: agora.${errorName}`;
                }
                
                // Try to decode using API
                if (api) {
                  try {
                    const errorMeta = api.registry.findMetaError({ 
                      index: moduleIndex, 
                      error: errorIndex 
                    });
                    if (errorMeta) {
                      errorMessage = `Error: ${errorMeta.section}.${errorMeta.name}`;
                      if (errorMeta.name === 'SaltVerificationFailed') {
                        errorMessage = 'Salt verification failed! The result you entered does not match the hash you committed. Make sure you enter the EXACT same result text you used when committing.';
                      }
                    }
                  } catch (e) {
                    console.error('Error decoding:', e);
                  }
                }
              } else if (dispatchError?.BadOrigin) {
                errorMessage = 'Bad origin: Unauthorized';
              } else if (dispatchError?.CannotLookup) {
                errorMessage = 'Cannot lookup: Account not found';
              } else if (dispatchError?.Other) {
                errorMessage = `Other error: ${dispatchError.Other}`;
              }
            } catch (e) {
              console.error('Error parsing failed event:', e);
              errorMessage = `Transaction failed: ${failed.event.data.toString()}`;
            }
            
            setIsRevealing(false);
            alert(errorMessage);
            return;
          }
          
          // Check for success event
          const revealed = events.find(e => 
            e.event.section === 'agora' && e.event.method === 'ResultRevealed'
          );
          if (revealed) {
            console.log('✅ ResultRevealed event found:', revealed.event.data.toHuman());
          }
          
          loadReveals(selectedJobId);
        }
        
        if (status.isFinalized) {
          const blockHash = status.asFinalized.toString();
          console.log('✅ Reveal transaction finalized in block:', blockHash);
          console.log('📊 Final events:', events.map(e => ({
            section: e.event.section,
            method: e.event.method,
            data: e.event.data.toHuman(),
          })));
          
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('❌ Reveal transaction failed:', failed);
            setIsRevealing(false);
            alert('Reveal transaction failed on-chain. Check console for details.');
            return;
          }
          
          setResult('');
          setIsRevealing(false);
          alert(`Reveal submitted successfully!\n\nBlock: ${blockHash}\n\nCheck Polkadot.js Apps to verify.`);
        }
      });
    } catch (error) {
      console.error('Failed to reveal:', error);
      setIsRevealing(false);
      alert(`Failed to reveal: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  if (!account) {
    return (
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <p className="text-gray-500">Please connect your wallet to commit/reveal results</p>
      </div>
    );
  }

  const selectedJob = selectedJobId !== null ? jobs.get(selectedJobId) : null;
  const selectedJobCommits = selectedJobId !== null ? commits.get(selectedJobId) || [] : [];
  const selectedJobReveals = selectedJobId !== null ? reveals.get(selectedJobId) || [] : [];
  const hasCommitted = selectedJobCommits.some(c => c.worker.toString() === account.address);
  const canCommit = selectedJob && (selectedJob.status.toString() === JobStatus.Pending || selectedJob.status.toString() === JobStatus.CommitPhase) && !hasCommitted;
  const canReveal = selectedJob && selectedJob.status.toString() === JobStatus.RevealPhase && hasCommitted && !selectedJobReveals.some(r => r.worker.toString() === account.address);
  const myCommit = selectedJobId !== null ? selectedJobCommits.find(c => c.worker.toString() === account.address) : null;

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6">
      <h2 className="text-xl font-semibold mb-4">Commit/Reveal Results</h2>
      
      <div className="mb-6">
        <label className="block text-sm font-medium text-gray-700 mb-2">
          Select Job
        </label>
        <select
          value={selectedJobId ?? ''}
          onChange={(e) => setSelectedJobId(e.target.value ? parseInt(e.target.value) : null)}
          className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
        >
          <option value="">Select a job...</option>
          {availableJobs.map(([jobId, job]) => (
            <option key={jobId} value={jobId}>
              Job #{jobId} - {formatBalance(job.bounty.toString())} UNIT
            </option>
          ))}
        </select>
      </div>

      {selectedJob && (
        <div className="space-y-4">
          <div className="bg-gray-50 p-4 rounded-lg">
            <p className="text-sm text-gray-500">Job Status</p>
            <p className="font-medium">{selectedJob.status.toString()}</p>
            <p className="text-sm text-gray-500 mt-2">Bounty</p>
            <p className="font-medium">{formatBalance(selectedJob.bounty.toString())} UNIT</p>
            <p className="text-sm text-gray-500 mt-2">Current Block</p>
            <p className="font-medium">{currentBlock}</p>
            <p className="text-sm text-gray-500 mt-2">Commit Deadline</p>
            <p className="font-medium">Block {selectedJob.commitDeadline.toString()}</p>
            <p className="text-sm text-gray-500 mt-2">Reveal Deadline</p>
            <p className="font-medium">Block {selectedJob.revealDeadline.toString()}</p>
          </div>

          {canCommit && (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Result (to commit)
                </label>
                <textarea
                  value={result}
                  onChange={(e) => setResult(e.target.value)}
                  placeholder="Enter job result..."
                  rows={4}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                />
              </div>
              <button
                onClick={handleGenerateCommit}
                className="w-full px-4 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 transition-colors"
              >
                Generate Commit Hash
              </button>
              {salt && (
                <div className="bg-gray-50 p-4 rounded-lg">
                  <p className="text-xs text-gray-500 mb-1">Salt (32 bytes)</p>
                  <p className="font-mono text-sm break-all">
                    0x{Array.from(salt).map(b => b.toString(16).padStart(2, '0')).join('')}
                  </p>
                </div>
              )}
              {commitHash && (
                <div className="bg-gray-50 p-4 rounded-lg">
                  <div className="flex items-center justify-between">
                    <div className="flex-1">
                      <p className="text-xs text-gray-500 mb-1">Commit Hash</p>
                      <p className="font-mono text-sm break-all">{commitHash}</p>
                    </div>
                    <button
                      onClick={handleCopyHash}
                      className="ml-2 p-2 text-gray-500 hover:text-gray-700 transition-colors"
                      title="Copy hash"
                    >
                      {copied ? (
                        <Check className="w-5 h-5 text-green-600" />
                      ) : (
                        <Copy className="w-5 h-5" />
                      )}
                    </button>
                  </div>
                </div>
              )}
              <button
                onClick={handleCommit}
                disabled={isCommitting || !commitHash || !salt}
                className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isCommitting ? (
                  <>
                    <Loader className="w-5 h-5 animate-spin" />
                    <span>Committing...</span>
                  </>
                ) : (
                  <>
                    <EyeOff className="w-5 h-5" />
                    <span>Commit Result</span>
                  </>
                )}
              </button>
            </div>
          )}

          {canReveal && (
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  Result (to reveal) - Must match committed result
                </label>
                <textarea
                  value={result}
                  onChange={(e) => setResult(e.target.value)}
                  placeholder="Enter the same result you committed..."
                  rows={4}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                />
                {selectedJobId !== null && account && (
                  <button
                    onClick={() => {
                      const storageKey = `agora_commit_${selectedJobId}_${account.address}`;
                      const storedResult = localStorage.getItem(storageKey);
                      if (storedResult) {
                        setResult(storedResult);
                        alert('Restored your committed result from storage!');
                      } else {
                        alert('No stored result found. Make sure you committed using this interface.');
                      }
                    }}
                    className="mt-2 px-3 py-1 text-xs bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition-colors"
                  >
                    Restore Committed Result
                  </button>
                )}
              </div>
              {myCommit && (
                <div className="bg-blue-50 border border-blue-200 p-4 rounded-lg">
                  <p className="text-xs text-blue-800 mb-2">⚠️ Important:</p>
                  <p className="text-xs text-blue-800 mb-2">
                    The result you enter must EXACTLY match the result you committed (including spaces, newlines, etc.). 
                    The system will verify the hash matches your committed hash.
                  </p>
                  <p className="text-xs text-blue-800">
                    <strong>Tip:</strong> If you're not sure what result you committed, click "Restore Committed Result" above 
                    or check your browser's console logs from when you committed.
                  </p>
                </div>
              )}
              <button
                onClick={handleReveal}
                disabled={isRevealing || !result.trim()}
                className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isRevealing ? (
                  <>
                    <Loader className="w-5 h-5 animate-spin" />
                    <span>Revealing...</span>
                  </>
                ) : (
                  <>
                    <Eye className="w-5 h-5" />
                    <span>Reveal Result</span>
                  </>
                )}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
