import { createContext, useContext } from 'react';
import type { ReactNode } from 'react';
import { ApiPromise } from '@polkadot/api';
import { useApi } from '../hooks/useApi';

interface ApiContextType {
  api: ApiPromise | null;
  isConnected: boolean;
  isLoading: boolean;
  error: Error | null;
  reconnect: (endpoint: string) => Promise<void>;
}

const ApiContext = createContext<ApiContextType | undefined>(undefined);

export function ApiProvider({ children, endpoint }: { children: ReactNode; endpoint?: string }) {
  const api = useApi(endpoint);

  return (
    <ApiContext.Provider value={api}>
      {children}
    </ApiContext.Provider>
  );
}

export function useApiContext() {
  const context = useContext(ApiContext);
  if (context === undefined) {
    throw new Error('useApiContext must be used within an ApiProvider');
  }
  return context;
}

