import React from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { ApiProvider } from './context/ApiContext';
import { AccountProvider } from './context/AccountContext';
import { Layout } from './components/layout/Layout';
import { Dashboard } from './components/Dashboard';
import { JobList } from './components/jobs/JobList';
import { JobDetail } from './components/jobs/JobDetail';
import { SubmitJobForm } from './components/jobs/SubmitJobForm';
import { WorkerDashboard } from './components/workers/WorkerDashboard';
import { CommitRevealInterface } from './components/workers/CommitRevealInterface';
import { Settings } from './components/Settings';
import { NotFound } from './components/NotFound';
import { DEFAULT_ENDPOINT } from './utils/constants';

function App() {
  return (
    <ApiProvider endpoint={DEFAULT_ENDPOINT}>
      <AccountProvider>
        <BrowserRouter>
          <Layout>
            <Routes>
              <Route path="/" element={<Dashboard />} />
              <Route path="/jobs" element={<JobList />} />
              <Route path="/jobs/:id" element={<JobDetail />} />
              <Route path="/jobs/submit" element={<SubmitJobForm />} />
              <Route path="/workers" element={<WorkerDashboard />} />
              <Route path="/workers/commit-reveal" element={<CommitRevealInterface />} />
              <Route path="/settings" element={<Settings />} />
              <Route path="*" element={<NotFound />} />
            </Routes>
          </Layout>
        </BrowserRouter>
      </AccountProvider>
    </ApiProvider>
  );
}

export default App;
