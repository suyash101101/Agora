import { useState, useEffect, useCallback } from 'react';
import { ApiPromise } from '@polkadot/api';
import { web3FromAddress, web3Enable, web3Accounts } from '@polkadot/extension-dapp';
import { Keyring } from '@polkadot/keyring';
import { cryptoWaitReady } from '@polkadot/util-crypto';
import type { AccountInfo } from '../types';
import { formatBalance } from '../utils/formatters';

/**
 * Helper function to get test account name and seed
 * These are well-known Substrate test accounts for local development
 */
function getTestAccountInfo(address: string): { name: string; seed: string } | null {
  const testAccounts: Record<string, { name: string; seed: string }> = {
    '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY': { name: 'Alice', seed: '//Alice' },
    '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty': { name: 'Bob', seed: '//Bob' },
    '5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y': { name: 'Charlie', seed: '//Charlie' },
  };
  return testAccounts[address] || null;
}

function getTestAccountName(address: string): string | undefined {
  const info = getTestAccountInfo(address);
  return info?.name;
}

/**
 * Custom hook for managing account state and wallet connections
 * 
 * Follows official Polkadot.js patterns:
 * @see https://polkadot.js.org/docs/extension/usage
 * @see https://polkadot.js.org/docs/extension/cookbook
 */
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

  /**
   * Connect to Polkadot.js extension
   * 
   * Pattern from official docs:
   * @see https://polkadot.js.org/docs/extension/cookbook#get-all-extensions-accounts
   */
  const connectExtension = useCallback(async () => {
    if (!api) return;

    try {
      // web3Enable must be called first before any other extension requests
      // @see https://polkadot.js.org/docs/extension/usage
      const extensions = await web3Enable('Agora Parachain UI');
      
      if (extensions.length === 0) {
        throw new Error('No Polkadot.js extension found. Please install the Polkadot.js extension.');
      }

      // Get all accounts from all enabled extensions
      const accounts = await web3Accounts();
      if (accounts.length === 0) {
        throw new Error('No accounts found in extension. Please add an account to your Polkadot.js extension, or use the "Test Account" button to connect as Alice, Bob, or Charlie.');
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

  /**
   * Connect a test account manually (for local development)
   * Uses well-known Substrate test accounts
   */
  const connectManual = useCallback((address: string) => {
    const accountInfo: AccountInfo = {
      address,
      name: getTestAccountName(address),
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

  /**
   * Get signer for an address
   * 
   * For test accounts: Creates a KeyringPair from seed
   * For extension accounts: Retrieves signer from extension
   * 
   * Pattern from official docs:
   * @see https://polkadot.js.org/docs/extension/usage
   * @see https://polkadot.js.org/docs/keyring
   */
  const getSigner = useCallback(async (address: string): Promise<any> => {
    // Check if this is a test account (for local testing)
    const testAccountInfo = getTestAccountInfo(address);
    if (testAccountInfo) {
      try {
        // Ensure crypto is ready before creating keyring
        await cryptoWaitReady();
        
        // Create keypair from keyring for test accounts
        // Use SS58 format 42 (Substrate default) for local testnet
        const keyring = new Keyring({ type: 'sr25519', ss58Format: 42 });
        const pair = keyring.addFromUri(testAccountInfo.seed);
        // Return the keypair directly - it implements the signer interface
        return pair;
      } catch (error) {
        console.error('Failed to create test account signer:', error);
        return null;
      }
    }

    // Otherwise, try to get signer from extension
    try {
      // Ensure extension is enabled first (required before web3FromAddress)
      // @see https://polkadot.js.org/docs/extension/usage
      await web3Enable('Agora Parachain UI');
      
      // Retrieve injector for the specific address
      // @see https://polkadot.js.org/docs/extension/usage
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

