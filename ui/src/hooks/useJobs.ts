import { useState, useEffect, useCallback } from 'react';
import { ApiPromise } from '@polkadot/api';
import type { Job, Commit, Reveal } from '../types';

export function useJobs(api: ApiPromise | null) {
  const [jobs, setJobs] = useState<Map<number, Job>>(new Map());
  const [commits, setCommits] = useState<Map<number, Commit[]>>(new Map());
  const [reveals, setReveals] = useState<Map<number, Reveal[]>>(new Map());
  const [results, setResults] = useState<Map<number, Uint8Array>>(new Map());
  const [isLoading, setIsLoading] = useState(false);

  const loadJob = useCallback(async (jobId: number) => {
    if (!api) return;

    try {
      const job = await api.query.agora.jobs(jobId);
      if ((job as any).isSome) {
        const jobData = (job as any).unwrap();
        setJobs(prev => new Map(prev).set(jobId, jobData as any));
      }
    } catch (error) {
      console.error(`Failed to load job ${jobId}:`, error);
    }
  }, [api]);

  const loadCommits = useCallback(async (jobId: number) => {
    if (!api) return;

    try {
      const commitsData = await api.query.agora.commits(jobId);
      if ((commitsData as any).isSome) {
        const commitsList = (commitsData as any).unwrap().toArray() as any[];
        // Log salt extraction for debugging
        commitsList.forEach((commit, idx) => {
          console.log(`🔍 Commit ${idx} salt extraction:`, {
            salt: commit.salt,
            saltType: typeof commit.salt,
            saltConstructor: commit.salt?.constructor?.name,
            saltToU8a: commit.salt?.toU8a?.(),
            saltToHex: commit.salt?.toHex?.(),
            saltIsArray: Array.isArray(commit.salt),
            saltIsUint8Array: commit.salt instanceof Uint8Array,
          });
        });
        setCommits(prev => new Map(prev).set(jobId, commitsList));
      }
    } catch (error) {
      console.error(`Failed to load commits for job ${jobId}:`, error);
    }
  }, [api]);

  const loadReveals = useCallback(async (jobId: number) => {
    if (!api) return;

    try {
      const revealsData = await api.query.agora.reveals(jobId);
      if ((revealsData as any).isSome) {
        const revealsList = (revealsData as any).unwrap().toArray() as any[];
        setReveals(prev => new Map(prev).set(jobId, revealsList));
      }
    } catch (error) {
      console.error(`Failed to load reveals for job ${jobId}:`, error);
    }
  }, [api]);

  const loadResult = useCallback(async (jobId: number) => {
    if (!api) return;

    try {
      const resultData = await api.query.agora.results(jobId);
      if ((resultData as any).isSome) {
        const result = (resultData as any).unwrap().toU8a();
        setResults(prev => new Map(prev).set(jobId, result));
      }
    } catch (error) {
      console.error(`Failed to load result for job ${jobId}:`, error);
    }
  }, [api]);

  const loadAllJobs = useCallback(async () => {
    if (!api) return;

    try {
      setIsLoading(true);
      const nextJobId = await api.query.agora.nextJobId();
      const jobIdNum = (nextJobId as any).toNumber();
      
      const jobsPromises = [];
      for (let i = 0; i < jobIdNum; i++) {
        jobsPromises.push(loadJob(i));
      }
      
      await Promise.all(jobsPromises);
    } catch (error) {
      console.error('Failed to load all jobs:', error);
    } finally {
      setIsLoading(false);
    }
  }, [api, loadJob]);

  useEffect(() => {
    if (api) {
      loadAllJobs();
      
      // Subscribe to events
      const unsubscribePromise: any = api.query.system.events((events: any) => {
        events.forEach((record: any) => {
          const { event } = record;
          if (api.events.agora.JobSubmitted.is(event)) {
            const jobId = (event.data[0] as any).toNumber();
            loadJob(jobId);
          } else if (api.events.agora.ResultCommitted.is(event)) {
            const jobId = (event.data[0] as any).toNumber();
            loadCommits(jobId);
          } else if (api.events.agora.ResultRevealed.is(event)) {
            const jobId = (event.data[0] as any).toNumber();
            loadReveals(jobId);
          } else if (api.events.agora.JobFinalized.is(event)) {
            const jobId = (event.data[0] as any).toNumber();
            loadJob(jobId);
            loadResult(jobId);
          }
        });
      });

      return () => {
        if (unsubscribePromise && typeof unsubscribePromise.then === 'function') {
          unsubscribePromise.then((unsub: any) => {
            if (typeof unsub === 'function') {
              unsub();
            }
          }).catch(() => {});
        } else if (typeof unsubscribePromise === 'function') {
          unsubscribePromise();
        }
      };
    }
  }, [api, loadJob, loadCommits, loadReveals, loadResult, loadAllJobs]);

  return {
    jobs,
    commits,
    reveals,
    results,
    isLoading,
    loadJob,
    loadCommits,
    loadReveals,
    loadResult,
    loadAllJobs,
  };
}

