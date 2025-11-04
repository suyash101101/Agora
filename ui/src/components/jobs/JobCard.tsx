import React from 'react';
import { Link } from 'react-router-dom';
import type { Job } from '../../types';
import { formatBalance, formatAddress, formatDeadline } from '../../utils/formatters';
import { JOB_STATUS_COLORS, JOB_TYPE_LABELS, JobStatus, JobType } from '../../utils/constants';
import { useApiContext } from '../../context/ApiContext';
import { Clock, Coins, User } from 'lucide-react';

interface JobCardProps {
  job: Job;
  jobId: number;
}

export function JobCard({ job, jobId }: JobCardProps) {
  const { api } = useApiContext();
  const [currentBlock, setCurrentBlock] = React.useState<number>(0);

  React.useEffect(() => {
    if (api) {
      api.rpc.chain.subscribeNewHeads((header) => {
        setCurrentBlock(header.number.toNumber());
      });
    }
  }, [api]);

  const status = job.status.toString() as JobStatus;
  const statusColor = JOB_STATUS_COLORS[status] || JOB_STATUS_COLORS[JobStatus.Pending];
  const jobTypeNum = Number(job.jobType);
  const jobType = (jobTypeNum === JobType.ApiRequest || jobTypeNum === JobType.Computation) 
    ? JOB_TYPE_LABELS[jobTypeNum as JobType]
    : 'Unknown';

  const deadline = job.status === JobStatus.CommitPhase 
    ? formatDeadline(job.commitDeadline.toNumber(), currentBlock)
    : job.status === JobStatus.RevealPhase
    ? formatDeadline(job.revealDeadline.toNumber(), currentBlock)
    : 'N/A';

  return (
    <Link to={`/jobs/${jobId}`}>
      <div className="bg-white rounded-lg border border-gray-200 p-6 hover:shadow-lg transition-shadow">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h3 className="text-lg font-semibold text-gray-900">Job #{jobId}</h3>
            <p className="text-sm text-gray-500 mt-1">{jobType}</p>
          </div>
          <span className={`px-3 py-1 rounded-full text-xs font-medium ${statusColor}`}>
            {status}
          </span>
        </div>
        
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-sm text-gray-600">
            <User className="w-4 h-4" />
            <span>{formatAddress(job.creator.toString())}</span>
          </div>
          <div className="flex items-center gap-2 text-sm text-gray-600">
            <Coins className="w-4 h-4" />
            <span>{formatBalance(job.bounty.toString())} UNIT</span>
          </div>
          {(job.status === JobStatus.CommitPhase || job.status === JobStatus.RevealPhase) && (
            <div className="flex items-center gap-2 text-sm text-gray-600">
              <Clock className="w-4 h-4" />
              <span>{deadline} remaining</span>
            </div>
          )}
        </div>
      </div>
    </Link>
  );
}

