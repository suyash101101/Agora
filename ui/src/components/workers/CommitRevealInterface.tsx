import React, { useState } from 'react';
import { useApiContext } from '../../context/ApiContext';
import { useAccountContext } from '../../context/AccountContext';
import { useJobs } from '../../hooks/useJobs';
import { generateSalt, hashResult, generateCommitHash } from '../../utils/helpers';
import { stringToBytes, bytesToString } from '../../utils/formatters';
import { formatBalance, formatAddress } from '../../utils/formatters';
import { JobStatus } from '../../utils/constants';
import { signAndSend } from '../../utils/signer';
import { Send, Loader, Eye, EyeOff } from 'lucide-react';

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

  const handleCommit = async () => {
    if (!api || !account || !selectedJobId || !salt || !commitHash) return;

    try {
      setIsCommitting(true);
      const signer = await getSigner(account.address);
      if (!signer) {
        alert('No signer available');
        setIsCommitting(false);
        return;
      }
      
      const tx = api.tx.agora.commitResult(selectedJobId, Array.from(salt), commitHash);
      await signAndSend(tx, signer, account.address, ({ status }) => {
        if (status.isInBlock) {
          loadCommits(selectedJobId);
          setResult('');
          setSalt(null);
          setCommitHash(null);
          setIsCommitting(false);
          alert('Commit submitted successfully!');
        } else if (status.isFinalized) {
          setIsCommitting(false);
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
      const signer = await getSigner(account.address);
      if (!signer) {
        alert('No signer available');
        setIsRevealing(false);
        return;
      }
      
      const resultBytes = stringToBytes(result);
      const tx = api.tx.agora.revealResult(selectedJobId, Array.from(resultBytes));
      await signAndSend(tx, signer, account.address, ({ status }) => {
        if (status.isInBlock) {
          loadReveals(selectedJobId);
          setResult('');
          setIsRevealing(false);
          alert('Reveal submitted successfully!');
        } else if (status.isFinalized) {
          setIsRevealing(false);
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
  const canCommit = selectedJob && selectedJob.status.toString() === JobStatus.CommitPhase && !hasCommitted;
  const canReveal = selectedJob && selectedJob.status.toString() === JobStatus.RevealPhase && hasCommitted && !selectedJobReveals.some(r => r.worker.toString() === account.address);

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
              {commitHash && (
                <div className="bg-gray-50 p-4 rounded-lg">
                  <p className="text-xs text-gray-500 mb-1">Commit Hash</p>
                  <p className="font-mono text-sm break-all">{commitHash}</p>
                </div>
              )}
              <button
                onClick={handleCommit}
                disabled={isCommitting || !commitHash}
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
                  Result (to reveal)
                </label>
                <textarea
                  value={result}
                  onChange={(e) => setResult(e.target.value)}
                  placeholder="Enter the same result you committed..."
                  rows={4}
                  className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
                />
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
    </div>
  );
}

