import { useEffect, useState, useCallback } from 'react';
import { ApiPromise } from '@polkadot/api';

export function useEvents(api: ApiPromise | null) {
  const [events, setEvents] = useState<any[]>([]);
  const [isSubscribed, setIsSubscribed] = useState(false);

  useEffect(() => {
    if (!api || !api.isConnected) {
      setIsSubscribed(false);
      return;
    }

    let unsubscribe: (() => void) | null = null;

    const subscribe = async () => {
      try {
        const unsub = await api.query.system.events((events: any) => {
          const agoraEvents = events
            .filter((record: any) => {
              const { event } = record;
              return event.section === 'agora';
            })
            .map((record: any) => ({
              blockNumber: record.block.header.number,
              event: record.event.toHuman(),
              phase: record.phase.toString(),
            }));

          setEvents(prev => [...agoraEvents, ...prev].slice(0, 100)); // Keep last 100 events
          setIsSubscribed(true);
        });
        
        if (typeof unsub === 'function') {
          unsubscribe = unsub;
        } else if (unsub && typeof (unsub as any).then === 'function') {
          unsubscribe = await (unsub as any) as () => void;
        } else {
          unsubscribe = unsub as any as () => void;
        }
      } catch (error) {
        console.error('Failed to subscribe to events:', error);
        setIsSubscribed(false);
      }
    };

    subscribe();

    return () => {
      if (unsubscribe) {
        unsubscribe();
      }
      setIsSubscribed(false);
    };
  }, [api]);

  const clearEvents = useCallback(() => {
    setEvents([]);
  }, []);

  return {
    events,
    isSubscribed,
    clearEvents,
  };
}

