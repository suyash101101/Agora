import React, { useState } from 'react';
import { useJobs } from '../../hooks/useJobs';
import { useApiContext } from '../../context/ApiContext';
import { JobCard } from './JobCard';
import { JobStatus, JOB_STATUS_COLORS } from '../../utils/constants';
import { Loader, Search, Filter } from 'lucide-react';

export function JobList() {
  const { api } = useApiContext();
  const { jobs, isLoading } = useJobs(api);
  const [filter, setFilter] = useState<JobStatus | 'all'>('all');
  const [searchTerm, setSearchTerm] = useState('');

  const filteredJobs = Array.from(jobs.entries()).filter(([jobId, job]) => {
    const matchesFilter = filter === 'all' || job.status.toString() === filter;
    const matchesSearch = jobId.toString().includes(searchTerm) || 
                         job.creator.toString().includes(searchTerm);
    return matchesFilter && matchesSearch;
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader className="w-8 h-8 animate-spin text-primary-600" />
      </div>
    );
  }

  return (
    <div>
      <div className="mb-6 flex flex-col sm:flex-row gap-4">
        <div className="flex-1 relative">
          <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-5 h-5 text-gray-400" />
          <input
            type="text"
            placeholder="Search jobs..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
        </div>
        <div className="flex items-center gap-2">
          <Filter className="w-5 h-5 text-gray-400" />
          <select
            value={filter}
            onChange={(e) => setFilter(e.target.value as JobStatus | 'all')}
            className="px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
          >
            <option value="all">All Status</option>
            {Object.values(JobStatus).map((status) => (
              <option key={status} value={status}>
                {status}
              </option>
            ))}
          </select>
        </div>
      </div>

      {filteredJobs.length === 0 ? (
        <div className="text-center py-12">
          <p className="text-gray-500">No jobs found</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {filteredJobs.map(([jobId, job]) => (
            <JobCard key={jobId} job={job} jobId={jobId} />
          ))}
        </div>
      )}
    </div>
  );
}

