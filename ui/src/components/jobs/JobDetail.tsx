import React, { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useJobs } from '../../hooks/useJobs';
import { useApiContext } from '../../context/ApiContext';
import { useAccountContext } from '../../context/AccountContext';
import { formatBalance, formatAddress, bytesToString } from '../../utils/formatters';
import { JOB_STATUS_COLORS, JOB_TYPE_LABELS, JobStatus, JobType } from '../../utils/constants';
import { canFinalizeJob } from '../../utils/helpers';
import { Loader, ArrowLeft, CheckCircle } from 'lucide-react';

export function JobDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { api } = useApiContext();
  const { account, getSigner } = useAccountContext();
  const { jobs, commits, reveals, results, loadJob, loadCommits, loadReveals } = useJobs(api);
  const [currentBlock, setCurrentBlock] = React.useState<number>(0);
  const [isFinalizing, setIsFinalizing] = React.useState(false);

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

  const handleFinalize = async () => {
    if (!api || !account) return;

    try {
      setIsFinalizing(true);
      const signer = await getSigner(account.address);
      const tx = api.tx.agora.finalizeJob(jobId);
      
      await tx.signAndSend(account.address, { signer }, ({ status }) => {
        if (status.isInBlock) {
          console.log('Finalized in block:', status.asInBlock.toString());
          loadJob(jobId);
          setIsFinalizing(false);
        } else if (status.isFinalized) {
          setIsFinalizing(false);
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
            <p className="text-sm text-gray-500">Created</p>
            <p className="font-medium">Block {job.createdAt.toString()}</p>
          </div>
          <div>
            <p className="text-sm text-gray-500">Commit Deadline</p>
            <p className="font-medium">Block {job.commitDeadline.toString()}</p>
          </div>
        </div>

        <div className="mb-6">
          <p className="text-sm text-gray-500 mb-2">Input Data</p>
          <div className="bg-gray-50 p-4 rounded-lg font-mono text-sm break-all">
            {bytesToString(job.inputData)}
          </div>
        </div>

        {canFinalize && job.status.toString() !== JobStatus.Completed && account && (
          <button
            onClick={handleFinalize}
            disabled={isFinalizing}
            className="w-full px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isFinalizing ? 'Finalizing...' : 'Finalize Job'}
          </button>
        )}
      </div>

      {jobCommits.length > 0 && (
        <div className="bg-white rounded-lg border border-gray-200 p-6 mb-6">
          <h2 className="text-lg font-semibold mb-4">Commits ({jobCommits.length})</h2>
          <div className="space-y-3">
            {jobCommits.map((commit, idx) => (
              <div key={idx} className="bg-gray-50 p-4 rounded-lg">
                <p className="text-sm font-medium">{formatAddress(commit.worker.toString())}</p>
                <p className="text-xs text-gray-500 mt-1">
                  Hash: {commit.resultHash.toString().slice(0, 16)}...
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
              <div key={idx} className="bg-gray-50 p-4 rounded-lg">
                <p className="text-sm font-medium">{formatAddress(reveal.worker.toString())}</p>
                <p className="text-xs text-gray-500 mt-1 break-all">
                  Result: {bytesToString(reveal.result)}
                </p>
              </div>
            ))}
          </div>
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

