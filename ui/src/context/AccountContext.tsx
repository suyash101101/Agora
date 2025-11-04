import { createContext, useContext } from 'react';
import type { ReactNode } from 'react';
import { useAccount } from '../hooks/useAccount';
import { useApiContext } from './ApiContext';
import type { AccountInfo } from '../types';

interface AccountContextType {
  account: AccountInfo | null;
  balance: string;
  isLoading: boolean;
  connectExtension: () => Promise<void>;
  connectManual: (address: string) => void;
  disconnect: () => void;
  loadBalance: (address: string) => Promise<void>;
  getSigner: (address: string) => Promise<any>;
}

const AccountContext = createContext<AccountContextType | undefined>(undefined);

export function AccountProvider({ children }: { children: ReactNode }) {
  const { api } = useApiContext();
  const account = useAccount(api);

  return (
    <AccountContext.Provider value={account}>
      {children}
    </AccountContext.Provider>
  );
}

export function useAccountContext() {
  const context = useContext(AccountContext);
  if (context === undefined) {
    throw new Error('useAccountContext must be used within an AccountProvider');
  }
  return context;
}

