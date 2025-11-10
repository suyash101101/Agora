import { useState, useEffect } from 'react';
import { useApiContext } from '../context/ApiContext';
import { useJobs } from '../hooks/useJobs';
import { useWorkers } from '../hooks/useWorkers';
import { JobStatus } from '../utils/constants';
import { Briefcase, Users, Coins, Award, Loader } from 'lucide-react';
import type { Job } from '../types';
import type { Header } from '@polkadot/types/interfaces';

export function Dashboard() {
  const { api } = useApiContext();
  const { jobs, isLoading: jobsLoading } = useJobs(api);
  const { workers, isLoading: workersLoading } = useWorkers(api);
  const [blockNumber, setBlockNumber] = useState<number>(0);

  useEffect(() => {
    if (api) {
      api.rpc.chain.subscribeNewHeads((header: Header) => {
        setBlockNumber(header.number.toNumber());
      });
    }
  }, [api]);

  const jobsArray = Array.from(jobs.values()) as Job[];
  const stats = {
    totalJobs: jobs.size,
    activeJobs: jobsArray.filter(j => 
      j.status.toString() !== JobStatus.Completed && j.status.toString() !== JobStatus.Failed
    ).length,
    completedJobs: jobsArray.filter(j => 
      j.status.toString() === JobStatus.Completed
    ).length,
    totalWorkers: workers.size,
    totalBounties: jobsArray.reduce((sum: bigint, job: Job) => 
      sum + BigInt(job.bounty.toString()), 0n
    ),
  };

  const statCards = [
    {
      title: 'Total Jobs',
      value: stats.totalJobs,
      icon: Briefcase,
      color: 'bg-blue-500',
    },
    {
      title: 'Active Jobs',
      value: stats.activeJobs,
      icon: Briefcase,
      color: 'bg-green-500',
    },
    {
      title: 'Completed Jobs',
      value: stats.completedJobs,
      icon: Briefcase,
      color: 'bg-purple-500',
    },
    {
      title: 'Total Workers',
      value: stats.totalWorkers,
      icon: Users,
      color: 'bg-orange-500',
    },
    {
      title: 'Total Bounties',
      value: stats.totalBounties.toString(),
      icon: Coins,
      color: 'bg-yellow-500',
      suffix: ' UNIT',
    },
    {
      title: 'Current Block',
      value: blockNumber,
      icon: Award,
      color: 'bg-indigo-500',
    },
  ];

  if (jobsLoading || workersLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader className="w-8 h-8 animate-spin text-primary-600" />
      </div>
    );
  }

  return (
    <div>
      <h1 className="text-3xl font-bold text-gray-900 mb-8">Dashboard</h1>
      
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 mb-8">
        {statCards.map((stat) => {
          const Icon = stat.icon;
          return (
            <div key={stat.title} className="bg-white rounded-lg border border-gray-200 p-6">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-gray-500 mb-1">{stat.title}</p>
                  <p className="text-2xl font-bold text-gray-900">
                    {stat.value}{stat.suffix || ''}
                  </p>
                </div>
                <div className={`${stat.color} p-3 rounded-lg`}>
                  <Icon className="w-6 h-6 text-white" />
                </div>
              </div>
            </div>
          );
        })}
      </div>

      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <h2 className="text-xl font-semibold mb-4">Quick Actions</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <a
            href="/jobs"
            className="p-4 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors"
          >
            <h3 className="font-medium mb-2">View All Jobs</h3>
            <p className="text-sm text-gray-500">Browse and manage all jobs</p>
          </a>
          <a
            href="/workers"
            className="p-4 border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors"
          >
            <h3 className="font-medium mb-2">Manage Workers</h3>
            <p className="text-sm text-gray-500">Register and manage workers</p>
          </a>
        </div>
      </div>
    </div>
  );
}

