import React from 'react';
import { useAccountContext } from '../../context/AccountContext';
import { formatAddress } from '../../utils/formatters';
import { Wallet, LogOut, Copy, Check, TestTube } from 'lucide-react';

export function ConnectButton() {
  const { account, connectExtension, connectManual, disconnect } = useAccountContext();
  const [copied, setCopied] = React.useState(false);

  // Well-known test accounts for local development
  const TEST_ACCOUNTS = {
    alice: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
    bob: '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
    charlie: '5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y',
  };

  const handleConnect = async () => {
    try {
      await connectExtension();
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      // Show user-friendly error message
      alert(errorMessage);
    }
  };

  const handleConnectTestAccount = (accountName: keyof typeof TEST_ACCOUNTS) => {
    connectManual(TEST_ACCOUNTS[accountName]);
  };

  const handleCopy = () => {
    if (account) {
      navigator.clipboard.writeText(account.address);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  if (account) {
    return (
      <div className="flex items-center gap-2">
        <div className="flex items-center gap-2 px-4 py-2 bg-gray-100 rounded-lg">
          <Wallet className="w-4 h-4" />
          <span className="text-sm font-medium">{formatAddress(account.address)}</span>
          {account.name && (
            <span className="text-xs text-gray-500">({account.name})</span>
          )}
          {account.source === 'manual' && (
            <span className="text-xs text-blue-600">(Test Account)</span>
          )}
        </div>
        <button
          onClick={handleCopy}
          className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
          title="Copy address"
        >
          {copied ? (
            <Check className="w-4 h-4 text-green-600" />
          ) : (
            <Copy className="w-4 h-4" />
          )}
        </button>
        <button
          onClick={disconnect}
          className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
          title="Disconnect"
        >
          <LogOut className="w-4 h-4" />
        </button>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2">
      <button
        onClick={handleConnect}
        className="flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
      >
        <Wallet className="w-4 h-4" />
        <span>Connect Wallet</span>
      </button>
      <div className="relative group">
        <button
          className="flex items-center gap-2 px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors"
          title="Use test account for local testing"
        >
          <TestTube className="w-4 h-4" />
          <span>Test Account</span>
        </button>
        <div className="absolute right-0 top-full mt-2 w-48 bg-white border border-gray-200 rounded-lg shadow-lg opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-50">
          <div className="py-2">
            <button
              onClick={() => handleConnectTestAccount('alice')}
              className="w-full text-left px-4 py-2 hover:bg-gray-100 text-sm"
            >
              Connect as Alice
            </button>
            <button
              onClick={() => handleConnectTestAccount('bob')}
              className="w-full text-left px-4 py-2 hover:bg-gray-100 text-sm"
            >
              Connect as Bob
            </button>
            <button
              onClick={() => handleConnectTestAccount('charlie')}
              className="w-full text-left px-4 py-2 hover:bg-gray-100 text-sm"
            >
              Connect as Charlie
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

