import { useState, useEffect, useCallback } from 'react';
import { ApiPromise } from '@polkadot/api';
import type { WorkerInfo } from '../types';

export function useWorkers(api: ApiPromise | null) {
  const [workers, setWorkers] = useState<Map<string, WorkerInfo>>(new Map());
  const [isLoading, setIsLoading] = useState(false);

  const loadWorker = useCallback(async (address: string) => {
    if (!api) return;

    try {
      const workerData = await api.query.agora.workers(address);
      if ((workerData as any).isSome) {
        const worker = (workerData as any).unwrap() as any;
        setWorkers(prev => new Map(prev).set(address, worker));
      }
    } catch (error) {
      console.error(`Failed to load worker ${address}:`, error);
    }
  }, [api]);

  const loadAllWorkers = useCallback(async () => {
    if (!api) return;

    try {
      setIsLoading(true);
      const workersEntries = await api.query.agora.workers.entries();
      const workersMap = new Map<string, WorkerInfo>();
      
      workersEntries.forEach(([key, value]) => {
        const address = key.args[0].toString();
        const worker = (value as any).unwrap() as any;
        workersMap.set(address, worker);
      });
      
      setWorkers(workersMap);
    } catch (error) {
      console.error('Failed to load all workers:', error);
    } finally {
      setIsLoading(false);
    }
  }, [api]);

  useEffect(() => {
    if (api) {
      loadAllWorkers();
      
      // Subscribe to events
      const unsubscribePromise: any = api.query.system.events((events: any) => {
        events.forEach((record: any) => {
          const { event } = record;
          if (api.events.agora.WorkerRegistered.is(event)) {
            const worker = event.data[0].toString();
            loadWorker(worker);
          } else if (api.events.agora.WorkerUnregistered.is(event)) {
            const worker = event.data[0].toString();
            setWorkers(prev => {
              const next = new Map(prev);
              next.delete(worker);
              return next;
            });
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
  }, [api, loadWorker, loadAllWorkers]);

  return {
    workers,
    isLoading,
    loadWorker,
    loadAllWorkers,
  };
}

