import { useState, useEffect, useCallback } from 'react';
import { ApiPromise } from '@polkadot/api';
import { web3FromAddress } from '@polkadot/extension-dapp';
import type { AccountInfo } from '../types';
import { formatBalance } from '../utils/formatters';

export function useAccount(api: ApiPromise | null) {
  const [account, setAccount] = useState<AccountInfo | null>(null);
  const [balance, setBalance] = useState<string>('0');
  const [isLoading, setIsLoading] = useState(false);

  const loadBalance = useCallback(async (address: string) => {
    if (!api) return;

    try {
      setIsLoading(true);
      const accountData = await api.query.system.account(address);
      const accountInfo = accountData as any;
      const freeBalance = accountInfo.data.free.toString();
      setBalance(formatBalance(freeBalance));
    } catch (error) {
      console.error('Failed to load balance:', error);
    } finally {
      setIsLoading(false);
    }
  }, [api]);

  const connectExtension = useCallback(async () => {
    if (!api) return;

    try {
      const { web3Accounts, web3Enable } = await import('@polkadot/extension-dapp');
      const extensions = await web3Enable('Agora Parachain UI');
      
      if (extensions.length === 0) {
        throw new Error('No Polkadot.js extension found');
      }

      const accounts = await web3Accounts();
      if (accounts.length === 0) {
        throw new Error('No accounts found in extension');
      }

      const selectedAccount = accounts[0];
      const accountInfo: AccountInfo = {
        address: selectedAccount.address,
        name: selectedAccount.meta.name,
        source: 'extension',
      };

      setAccount(accountInfo);
      await loadBalance(selectedAccount.address);
    } catch (error) {
      console.error('Failed to connect extension:', error);
      throw error;
    }
  }, [api, loadBalance]);

  const connectManual = useCallback((address: string) => {
    const accountInfo: AccountInfo = {
      address,
      source: 'manual',
    };
    setAccount(accountInfo);
    loadBalance(address);
  }, [loadBalance]);

  const disconnect = useCallback(() => {
    setAccount(null);
    setBalance('0');
  }, []);

  useEffect(() => {
    if (account && api) {
      loadBalance(account.address);
      const interval = setInterval(() => {
        loadBalance(account.address);
      }, 10000); // Update every 10 seconds

      return () => clearInterval(interval);
    }
  }, [account, api, loadBalance]);

  const getSigner = useCallback(async (address: string) => {
    try {
      const injector = await web3FromAddress(address);
      return injector.signer;
    } catch (error) {
      console.error('Failed to get signer:', error);
      return null;
    }
  }, []);

  return {
    account,
    balance,
    isLoading,
    connectExtension,
    connectManual,
    disconnect,
    loadBalance,
    getSigner,
  };
}

