import { useEffect, useState, useCallback } from 'react';
import { ApiPromise } from '@polkadot/api';
import { createApi, disconnectApi } from '../utils/api';
import { DEFAULT_ENDPOINT } from '../utils/constants';

export function useApi(endpoint: string = DEFAULT_ENDPOINT) {
  const [api, setApi] = useState<ApiPromise | null>(null);
  const [isConnected, setIsConnected] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let isMounted = true;

    const connect = async () => {
      try {
        setIsLoading(true);
        setError(null);
        const apiInstance = await createApi(endpoint);
        
        if (isMounted) {
          setApi(apiInstance);
          setIsConnected(apiInstance.isConnected);
          setIsLoading(false);
        }
      } catch (err) {
        if (isMounted) {
          setError(err instanceof Error ? err : new Error('Failed to connect to API'));
          setIsLoading(false);
          setIsConnected(false);
        }
      }
    };

    connect();

    return () => {
      isMounted = false;
    };
  }, [endpoint]);

  const reconnect = useCallback(async (newEndpoint: string) => {
    try {
      setIsLoading(true);
      setError(null);
      await disconnectApi();
      const apiInstance = await createApi(newEndpoint);
      setApi(apiInstance);
      setIsConnected(apiInstance.isConnected);
      setIsLoading(false);
    } catch (err) {
      setError(err instanceof Error ? err : new Error('Failed to reconnect'));
      setIsLoading(false);
      setIsConnected(false);
    }
  }, []);

  return {
    api,
    isConnected,
    isLoading,
    error,
    reconnect,
  };
}

