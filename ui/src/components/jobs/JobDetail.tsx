import React, { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useJobs } from '../../hooks/useJobs';
import { useApiContext } from '../../context/ApiContext';
import { useAccountContext } from '../../context/AccountContext';
import { formatBalance, formatAddress, bytesToString, stringToBytes } from '../../utils/formatters';
import { JOB_STATUS_COLORS, JOB_TYPE_LABELS, JobStatus, JobType } from '../../utils/constants';
import { canFinalizeJob } from '../../utils/helpers';
import { generateSalt, generateCommitHash, generateCommitHashBytes, ensureSalt32Bytes, hashHexToHash } from '../../utils/helpers';
import { signAndSend } from '../../utils/signer';
import { Loader, ArrowLeft, CheckCircle, Eye, EyeOff, Copy, Check } from 'lucide-react';
import { hexToU8a } from '@polkadot/util';

export function JobDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { api } = useApiContext();
  const { account, getSigner } = useAccountContext();
  const { jobs, commits, reveals, results, loadJob, loadCommits, loadReveals } = useJobs(api);
  const [currentBlock, setCurrentBlock] = React.useState<number>(0);
  const [isFinalizing, setIsFinalizing] = React.useState(false);
  
  // Commit/Reveal state
  const [result, setResult] = useState('');
  const [salt, setSalt] = useState<Uint8Array | null>(null);
  const [commitHash, setCommitHash] = useState<string | null>(null);
  const [isCommitting, setIsCommitting] = useState(false);
  const [isRevealing, setIsRevealing] = useState(false);
  const [copied, setCopied] = useState(false);

  const jobId = id ? parseInt(id, 10) : -1;
  const job = jobs.get(jobId);
  const jobCommits = commits.get(jobId) || [];
  const jobReveals = reveals.get(jobId) || [];
  const jobResult = results.get(jobId);

  React.useEffect(() => {
    if (api && jobId >= 0) {
      loadJob(jobId);
      loadCommits(jobId);
      loadReveals(jobId);
      
      api.rpc.chain.subscribeNewHeads((header) => {
        setCurrentBlock(header.number.toNumber());
      });
    }
  }, [api, jobId, loadJob, loadCommits, loadReveals]);

  // Check if user can commit or reveal
  const myCommit = jobCommits.find(c => c.worker.toString() === account?.address);
  const hasCommitted = !!myCommit;
  const hasRevealed = jobReveals.some(r => r.worker.toString() === account?.address);
  
  const canCommit = job && account && 
    (job.status.toString() === JobStatus.Pending || job.status.toString() === JobStatus.CommitPhase) &&
    !hasCommitted && 
    currentBlock <= job.commitDeadline.toNumber();
  
  const canReveal = job && account && 
    job.status.toString() === JobStatus.RevealPhase &&
    hasCommitted &&
    !hasRevealed &&
    currentBlock > job.commitDeadline.toNumber() &&
    currentBlock <= job.revealDeadline.toNumber();

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
    if (!api || !account || !salt || !commitHash) return;

    try {
      setIsCommitting(true);
      
      const salt32 = ensureSalt32Bytes(salt);
      const saltArray = Array.from(salt32);
      const hashHex = hashHexToHash(commitHash);
      
      // Convert salt to hex string for storage
      const saltHex = Array.from(salt32).map(b => b.toString(16).padStart(2, '0')).join('');
      
      console.log('🔐 Committing result:', {
        jobId,
        saltArray: saltArray,
        saltLength: saltArray.length,
        saltHex: saltHex,
        commitHash: hashHex,
        result: result,
        resultLength: result.length,
      });

      const signer = await getSigner(account.address);
      if (!signer) {
        alert('No signer available');
        setIsCommitting(false);
        return;
      }
      
      const tx = api.tx.agora.commitResult(jobId, saltArray, hashHex);
      
      await signAndSend(tx, signer, account.address, ({ status, events }) => {
        if (status.isInBlock) {
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('❌ Commit transaction failed:', failed);
            setIsCommitting(false);
            alert('Commit transaction failed. Check console for details.');
            return;
          }
          loadCommits(jobId);
        }
        
        if (status.isFinalized) {
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            setIsCommitting(false);
            alert('Commit transaction failed on-chain. Check console for details.');
            return;
          }
          
          // Store result AND salt in localStorage for reveal
          const storageKey = `agora_commit_${jobId}_${account.address}`;
          const saltKey = `agora_salt_${jobId}_${account.address}`;
          localStorage.setItem(storageKey, result);
          localStorage.setItem(saltKey, saltHex);
          
          console.log('💾 Stored commit data:', {
            result: result,
            saltHex: saltHex,
            storageKey: storageKey,
            saltKey: saltKey,
          });
          
          setResult('');
          setSalt(null);
          setCommitHash(null);
          setIsCommitting(false);
          loadJob(jobId);
          loadCommits(jobId);
          alert('Commit submitted successfully!');
        }
      });
    } catch (error) {
      console.error('Failed to commit:', error);
      setIsCommitting(false);
      alert(`Failed to commit: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const handleReveal = async () => {
    if (!api || !account || !result.trim()) return;

    if (!myCommit) {
      alert('You must commit first before revealing');
      return;
    }

    try {
      setIsRevealing(true);
      
      const resultBytes = stringToBytes(result);
      
      // CRITICAL: Query commit directly from chain RIGHT BEFORE revealing
      // This ensures we use the EXACT same salt the runtime will use
      console.log('🔍 Querying commit directly from chain to get exact salt...');
      const directCommit = await api.query.agora.commits(jobId);
      if (!(directCommit as any).isSome) {
        alert('No commits found for this job');
        setIsRevealing(false);
        return;
      }
      
      const commitsList = (directCommit as any).unwrap().toArray() as any[];
      const myDirectCommit = commitsList.find((c: any) => 
        c.worker.toString() === account.address
      );
      
      if (!myDirectCommit) {
        alert('You have not committed to this job');
        setIsRevealing(false);
        return;
      }
      
      console.log('🔍 Direct commit from chain:', myDirectCommit);
      console.log('🔍 Direct commit salt:', myDirectCommit.salt);
      
      // Extract salt from the direct commit query - this is the EXACT salt stored on-chain
      const directSalt = myDirectCommit.salt;
      let saltU8a: Uint8Array;
      
      // Try toHuman() first - most reliable for fixed arrays
      if (directSalt?.toHuman && typeof directSalt.toHuman === 'function') {
        const saltHuman = directSalt.toHuman();
        console.log('🔍 toHuman() returned:', saltHuman);
        
        if (Array.isArray(saltHuman)) {
          saltU8a = Uint8Array.from(saltHuman.slice(0, 32).map((v: any) => {
            if (typeof v === 'number') return v;
            if (typeof v === 'string') {
              if (v.startsWith('0x')) return parseInt(v.slice(2), 16);
              return parseInt(v, 10);
            }
            return parseInt(String(v), 10);
          }));
          console.log('✅ Extracted salt from toHuman() array:', Array.from(saltU8a));
        }
      }
      
      // If toHuman didn't work, try toU8a()
      if (!saltU8a && directSalt?.toU8a && typeof directSalt.toU8a === 'function') {
        const saltBytes = directSalt.toU8a();
        console.log('🔍 toU8a() returned:', saltBytes, 'type:', typeof saltBytes, 'length:', saltBytes?.length);
        
        if (saltBytes instanceof Uint8Array) {
          // For fixed arrays, toU8a() might return exactly 32 bytes or more
          if (saltBytes.length === 32) {
            saltU8a = saltBytes;
          } else if (saltBytes.length > 32) {
            // Might be encoded - try last 32 bytes
            saltU8a = saltBytes.slice(-32);
            console.log('✅ Extracted last 32 bytes from toU8a()');
          } else {
            // Too short - pad it
            const padded = new Uint8Array(32);
            padded.set(saltBytes);
            saltU8a = padded;
          }
        } else if (Array.isArray(saltBytes)) {
          saltU8a = Uint8Array.from(saltBytes.slice(0, 32));
        }
        if (saltU8a) {
          console.log('✅ Extracted salt from toU8a()');
        }
      }
      
      // Try direct array access
      if (!saltU8a && Array.isArray(directSalt)) {
        saltU8a = Uint8Array.from(directSalt.slice(0, 32));
        console.log('✅ Extracted salt as Array');
      }
      
      // Try Uint8Array direct access
      if (!saltU8a && directSalt instanceof Uint8Array) {
        saltU8a = directSalt.length === 32 ? directSalt : directSalt.slice(0, 32);
        console.log('✅ Extracted salt as Uint8Array');
      }
      
      // Try toHex()
      if (!saltU8a && directSalt?.toHex && typeof directSalt.toHex === 'function') {
        const hex = directSalt.toHex();
        saltU8a = hexToU8a(hex.startsWith('0x') ? hex : '0x' + hex);
        console.log('✅ Extracted salt from toHex()');
      }
      
      // Final check - ensure we have a valid salt
      if (!saltU8a || saltU8a.length !== 32) {
        console.error('❌ Failed to extract valid salt from chain commit');
        console.error('  Direct salt:', directSalt);
        console.error('  Salt type:', typeof directSalt);
        console.error('  Salt constructor:', directSalt?.constructor?.name);
        alert('Failed to extract salt from chain commit. Please try again.');
        setIsRevealing(false);
        return;
      }
      
      console.log('🔑 Final salt extracted from chain:', {
        saltHex: Array.from(saltU8a).map(b => b.toString(16).padStart(2, '0')).join(''),
        saltBytes: Array.from(saltU8a),
        saltLength: saltU8a.length,
      });
      
      // Verify against localStorage if available
      const saltKey = `agora_salt_${jobId}_${account.address}`;
      const storedSaltHex = localStorage.getItem(saltKey);
      if (storedSaltHex && storedSaltHex.length === 64) {
        const storedSalt = Uint8Array.from(
          storedSaltHex.match(/.{1,2}/g)!.map(byte => parseInt(byte, 16))
        );
        const match = saltU8a.length === storedSalt.length && saltU8a.every((b, i) => b === storedSalt[i]);
        if (match) {
          console.log('✅ Chain salt matches localStorage salt');
        } else {
          console.warn('⚠️ Chain salt does NOT match localStorage salt! Using chain salt.');
          console.warn('  Chain salt:', Array.from(saltU8a).map(b => b.toString(16).padStart(2, '0')).join(''));
          console.warn('  Stored salt:', Array.from(storedSalt).map(b => b.toString(16).padStart(2, '0')).join(''));
        }
      }
      
      // Calculate hash locally for verification
      const calculatedHash = generateCommitHashBytes(saltU8a, resultBytes);
      
      // Get committed hash from the direct commit query (same as runtime will use)
      const committedHashHex = myDirectCommit.resultHash.toString();
      let committedHashBytes: Uint8Array;
      try {
        committedHashBytes = hexToU8a(committedHashHex.startsWith('0x') ? committedHashHex : '0x' + committedHashHex);
      } catch (e) {
        console.error('Failed to convert committed hash:', e);
        committedHashBytes = new Uint8Array(32);
      }
      
      // Detailed logging
      console.log('🔓 Revealing result - Detailed verification:');
      console.log('  Job ID:', jobId);
      console.log('  Result (string):', JSON.stringify(result));
      console.log('  Result (bytes):', Array.from(resultBytes));
      console.log('  Result length:', resultBytes.length);
      console.log('  Salt (bytes):', Array.from(saltU8a));
      console.log('  Salt length:', saltU8a.length);
      console.log('  Salt (hex):', Array.from(saltU8a).map(b => b.toString(16).padStart(2, '0')).join(''));
      console.log('  Calculated hash (bytes):', Array.from(calculatedHash));
      console.log('  Calculated hash (hex):', Array.from(calculatedHash).map(b => b.toString(16).padStart(2, '0')).join(''));
      console.log('  Committed hash (hex):', committedHashHex);
      console.log('  Committed hash (bytes):', Array.from(committedHashBytes));
      
      // Verify hash matches before submitting
      const hashMatches = calculatedHash.length === committedHashBytes.length &&
        calculatedHash.every((b, i) => b === committedHashBytes[i]);
      
      console.log('  Hash match:', hashMatches);
      
      if (!hashMatches) {
        console.error('❌ Hash mismatch detected!');
        console.error('  Calculated:', Array.from(calculatedHash).map(b => b.toString(16).padStart(2, '0')).join(''));
        console.error('  Committed:', Array.from(committedHashBytes).map(b => b.toString(16).padStart(2, '0')).join(''));
        
        alert(`⚠️ Hash verification failed before submission!\n\n` +
              `This means the result you entered doesn't match what was committed.\n\n` +
              `Check the console for detailed comparison.\n\n` +
              `Common issues:\n` +
              `- Extra spaces or newlines\n` +
              `- Different encoding\n` +
              `- Salt mismatch\n\n` +
              `Try using "Restore Committed Result" button.`);
        setIsRevealing(false);
        return;
      }
      
      console.log('✅ Hash verification passed locally - proceeding with reveal');

      const signer = await getSigner(account.address);
      if (!signer) {
        alert('No signer available');
        setIsRevealing(false);
        return;
      }
      
      const tx = api.tx.agora.revealResult(jobId, Array.from(resultBytes));
      
      await signAndSend(tx, signer, account.address, ({ status, events }) => {
        if (status.isInBlock) {
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('❌ Reveal transaction failed:', failed);
            
            // Extract error details
            let errorMessage = 'Reveal transaction failed';
            try {
              const errorData = failed.event.data;
              const errorInfo = errorData.toHuman() as any;
              console.error('❌ Full error info:', errorInfo);
              console.error('❌ Error data (toJSON):', errorData.toJSON());
              
              const dispatchError = errorInfo.dispatchError || (Array.isArray(errorInfo) ? errorInfo[0] : errorInfo);
              console.error('❌ Dispatch error:', dispatchError);
              
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
                
                console.error(`❌ Module Index: ${moduleIndex}, Error Index: ${errorIndex} (from ${errorIndexHex})`);
                
                if (errorIndex === 17) { // SaltVerificationFailed
                  errorMessage = 'Salt verification failed! The result you entered does not match the hash you committed. Make sure you enter the EXACT same result text you used when committing.';
                  
                  // Log detailed comparison for debugging
                  console.error('❌ Salt verification failed on-chain!');
                  console.error('  Result used:', JSON.stringify(result));
                  console.error('  Salt used (hex):', Array.from(saltU8a).map(b => b.toString(16).padStart(2, '0')).join(''));
                  console.error('  Calculated hash (hex):', Array.from(calculatedHash).map(b => b.toString(16).padStart(2, '0')).join(''));
                  console.error('  Committed hash (hex):', committedHashHex);
                } else {
                  errorMessage = `Error: agora.${['JobNotFound', 'WorkerNotRegistered', 'WorkerAlreadyRegistered', 'InsufficientStake', 'InsufficientBounty', 'InvalidJobPhase', 'CommitMismatch', 'AlreadyCommitted', 'NotCommitted', 'CommitDeadlinePassed', 'RevealDeadlinePassed', 'JobAlreadyFinalized', 'InsufficientReveals', 'InputDataTooLarge', 'InsufficientBalance', 'WorkerUnbonding', 'TooManyConcurrentJobs', 'SaltVerificationFailed', 'AlreadyRevealed'][errorIndex] || `Unknown(${errorIndex})`}`;
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
            }
            
            setIsRevealing(false);
            alert(errorMessage);
            return;
          }
          
          loadReveals(jobId);
        }
        
        if (status.isFinalized) {
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            setIsRevealing(false);
            alert('Reveal transaction failed on-chain. Check console for details.');
            return;
          }
          
          setResult('');
          setIsRevealing(false);
          loadJob(jobId);
          loadReveals(jobId);
          alert('Reveal submitted successfully!');
        }
      });
    } catch (error) {
      console.error('Failed to reveal:', error);
      setIsRevealing(false);
      alert(`Failed to reveal: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const handleFinalize = async () => {
    if (!api || !account) return;

    try {
      setIsFinalizing(true);
      const signer = await getSigner(account.address);
      if (!signer) {
        alert('No signer available');
        setIsFinalizing(false);
        return;
      }
      
      const tx = api.tx.agora.finalizeJob(jobId);
      
      await signAndSend(tx, signer, account.address, ({ status, events }) => {
        if (status.isInBlock) {
          const failed = events.find(e => e.event.method === 'ExtrinsicFailed');
          if (failed) {
            console.error('❌ Finalize transaction failed:', failed);
            setIsFinalizing(false);
            alert('Finalize transaction failed. Check console for details.');
            return;
          }
        }
        
        if (status.isFinalized) {
          loadJob(jobId);
          setIsFinalizing(false);
          alert('Job finalized successfully!');
        }
      });
    } catch (error) {
      console.error('Failed to finalize job:', error);
      setIsFinalizing(false);
      alert(`Failed to finalize job: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  if (!job) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader className="w-8 h-8 animate-spin text-primary-600" />
      </div>
    );
  }

  const status = job.status.toString() as JobStatus;
  const statusColor = JOB_STATUS_COLORS[status] || JOB_STATUS_COLORS[JobStatus.Pending];
  const jobTypeNum = Number(job.jobType);
  const jobType = (jobTypeNum === JobType.ApiRequest || jobTypeNum === JobType.Computation)
    ? JOB_TYPE_LABELS[jobTypeNum as JobType]
    : 'Unknown';
  const canFinalize = canFinalizeJob(currentBlock, job.revealDeadline.toNumber());

  return (
    <div className="max-w-4xl mx-auto">
      <button
        onClick={() => navigate('/jobs')}
        className="flex items-center gap-2 text-gray-600 hover:text-gray-900 mb-6"
      >
        <ArrowLeft className="w-5 h-5" />
        <span>Back to Jobs</span>
      </button>

      <div className="bg-white rounded-lg border border-gray-200 p-6 mb-6">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">Job #{jobId}</h1>
            <p className="text-sm text-gray-500 mt-1">{jobType}</p>
          </div>
          <span className={`px-3 py-1 rounded-full text-xs font-medium ${statusColor}`}>
            {status}
          </span>
        </div>

        <div className="grid grid-cols-2 gap-4 mb-6">
          <div>
            <p className="text-sm text-gray-500">Creator</p>
            <p className="font-medium">{formatAddress(job.creator.toString())}</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">Bounty</p>
            <p className="font-medium">{formatBalance(job.bounty.toString())} UNIT</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">Current Block</p>
            <p className="font-medium">{currentBlock}</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">Commit Deadline</p>
            <p className="font-medium">Block {job.commitDeadline.toString()}</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">Reveal Deadline</p>
            <p className="font-medium">Block {job.revealDeadline.toString()}</p>
          </div>
        </div>

        <div className="mb-6">
          <p className="text-sm text-gray-500 mb-2">Input Data</p>
          <div className="bg-gray-50 p-4 rounded-lg font-mono text-sm break-all">
            {bytesToString(job.inputData)}
          </div>
        </div>
      </div>

      {/* Commit/Reveal Interface */}
      {account && (canCommit || canReveal) && (
        <div className="bg-white rounded-lg border border-gray-200 p-6 mb-6">
          <h2 className="text-lg font-semibold mb-4">
            {canCommit ? 'Commit Result' : 'Reveal Result'}
          </h2>
          
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
                <div className="flex gap-2 mt-2">
                  <button
                    onClick={() => {
                      const storageKey = `agora_commit_${jobId}_${account.address}`;
                      const storedResult = localStorage.getItem(storageKey);
                      if (storedResult) {
                        setResult(storedResult);
                        console.log('✅ Restored result from localStorage:', storedResult);
                        alert('Restored your committed result from storage!');
                      } else {
                        alert('No stored result found. Make sure you committed using this interface.');
                      }
                    }}
                    className="px-3 py-1 text-xs bg-gray-100 text-gray-700 rounded hover:bg-gray-200 transition-colors"
                  >
                    Restore Result
                  </button>
                  <button
                    onClick={async () => {
                      const saltKey = `agora_salt_${jobId}_${account.address}`;
                      const storedSaltHex = localStorage.getItem(saltKey);
                      const storageKey = `agora_commit_${jobId}_${account.address}`;
                      const storedResult = localStorage.getItem(storageKey);
                      
                      if (storedSaltHex && storedResult && myCommit) {
                        const storedSalt = Uint8Array.from(
                          storedSaltHex.match(/.{1,2}/g)!.map(byte => parseInt(byte, 16))
                        );
                        const resultBytes = stringToBytes(storedResult);
                        const calculatedHash = generateCommitHashBytes(storedSalt, resultBytes);
                        const committedHashHex = myCommit.resultHash.toString();
                        const committedHashBytes = hexToU8a(committedHashHex.startsWith('0x') ? committedHashHex : '0x' + committedHashHex);
                        
                        console.log('🔍 Verification with stored salt:');
                        console.log('  Stored salt (hex):', storedSaltHex);
                        console.log('  Stored salt (bytes):', Array.from(storedSalt));
                        console.log('  Stored result:', JSON.stringify(storedResult));
                        console.log('  Calculated hash (hex):', Array.from(calculatedHash).map(b => b.toString(16).padStart(2, '0')).join(''));
                        console.log('  Committed hash (hex):', Array.from(committedHashBytes).map(b => b.toString(16).padStart(2, '0')).join(''));
                        
                        const match = Array.from(calculatedHash).every((b, i) => b === committedHashBytes[i]);
                        console.log('  Match:', match);
                        
                        if (match) {
                          setResult(storedResult);
                          alert('✅ Hash verification passed with stored salt! Result restored.');
                        } else {
                          alert('❌ Hash verification failed even with stored salt. Check console for details.');
                        }
                      } else {
                        alert('No stored salt/result found. Make sure you committed using this interface.');
                      }
                    }}
                    className="px-3 py-1 text-xs bg-blue-100 text-blue-700 rounded hover:bg-blue-200 transition-colors"
                  >
                    Verify & Restore
                  </button>
                </div>
              </div>
              <div className="bg-blue-50 border border-blue-200 p-4 rounded-lg">
                <p className="text-xs text-blue-800 mb-2">⚠️ Important:</p>
                <p className="text-xs text-blue-800">
                  The result you enter must EXACTLY match the result you committed (including spaces, newlines, etc.). 
                  The system will verify the hash matches your committed hash.
                </p>
              </div>
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

      {/* Status info for workers */}
      {account && !canCommit && !canReveal && (
        <div className="bg-gray-50 border border-gray-200 p-4 rounded-lg mb-6">
          {hasCommitted && !hasRevealed && job.status.toString() !== JobStatus.RevealPhase && (
            <p className="text-sm text-gray-600">
              ✓ You have committed. Waiting for reveal phase to start...
            </p>
          )}
          {hasCommitted && hasRevealed && (
            <p className="text-sm text-gray-600">
              ✓ You have committed and revealed your result.
            </p>
          )}
          {!hasCommitted && job.status.toString() === JobStatus.RevealPhase && (
            <p className="text-sm text-gray-600">
              ⚠️ You did not commit a result for this job.
            </p>
          )}
        </div>
      )}

      {jobCommits.length > 0 && (
        <div className="bg-white rounded-lg border border-gray-200 p-6 mb-6">
          <h2 className="text-lg font-semibold mb-4">Commits ({jobCommits.length})</h2>
          <div className="space-y-3">
            {jobCommits.map((commit, idx) => (
              <div key={idx} className={`bg-gray-50 p-4 rounded-lg ${commit.worker.toString() === account?.address ? 'border-2 border-primary-500' : ''}`}>
                <p className="text-sm font-medium">
                  {formatAddress(commit.worker.toString())}
                  {commit.worker.toString() === account?.address && ' (You)'}
                </p>
                <p className="text-xs text-gray-500 mt-1">
                  Hash: {commit.resultHash.toString().slice(0, 16)}...
                </p>
                <p className="text-xs text-gray-500 mt-1">
                  Committed at block {commit.committedAt.toString()}
                </p>
              </div>
            ))}
          </div>
        </div>
      )}

      {jobReveals.length > 0 && (
        <div className="bg-white rounded-lg border border-gray-200 p-6 mb-6">
          <h2 className="text-lg font-semibold mb-4">Reveals ({jobReveals.length})</h2>
          <div className="space-y-3">
            {jobReveals.map((reveal, idx) => (
              <div key={idx} className={`bg-gray-50 p-4 rounded-lg ${reveal.worker.toString() === account?.address ? 'border-2 border-primary-500' : ''}`}>
                <p className="text-sm font-medium">
                  {formatAddress(reveal.worker.toString())}
                  {reveal.worker.toString() === account?.address && ' (You)'}
                </p>
                <p className="text-xs text-gray-500 mt-1 break-all">
                  Result: {bytesToString(reveal.result)}
                </p>
                <p className="text-xs text-gray-500 mt-1">
                  Revealed at block {reveal.revealedAt.toString()}
                </p>
              </div>
            ))}
          </div>
        </div>
      )}

      {canFinalize && job.status.toString() !== JobStatus.Completed && account && (
        <div className="bg-white rounded-lg border border-gray-200 p-6 mb-6">
          <h2 className="text-lg font-semibold mb-4">Finalize Job</h2>
          <p className="text-sm text-gray-600 mb-4">
            The reveal deadline has passed. You can now finalize this job to determine the final result.
          </p>
          <button
            onClick={handleFinalize}
            disabled={isFinalizing}
            className="w-full px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isFinalizing ? (
              <>
                <Loader className="w-5 h-5 animate-spin inline mr-2" />
                Finalizing...
              </>
            ) : (
              'Finalize Job'
            )}
          </button>
        </div>
      )}

      {jobResult && (
        <div className="bg-white rounded-lg border border-gray-200 p-6">
          <h2 className="text-lg font-semibold mb-4 flex items-center gap-2">
            <CheckCircle className="w-5 h-5 text-green-600" />
            Final Result
          </h2>
          <div className="bg-green-50 p-4 rounded-lg">
            <p className="font-mono text-sm break-all">{bytesToString(jobResult)}</p>
          </div>
        </div>
      )}
    </div>
  );
}
