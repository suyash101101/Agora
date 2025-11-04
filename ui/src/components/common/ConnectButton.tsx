import React from 'react';
import { useAccountContext } from '../../context/AccountContext';
import { formatAddress } from '../../utils/formatters';
import { Wallet, LogOut, Copy, Check } from 'lucide-react';

export function ConnectButton() {
  const { account, connectExtension, disconnect } = useAccountContext();
  const [copied, setCopied] = React.useState(false);

  const handleConnect = async () => {
    try {
      await connectExtension();
    } catch (error) {
      alert(`Failed to connect: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
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
    <button
      onClick={handleConnect}
      className="flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
    >
      <Wallet className="w-4 h-4" />
      <span>Connect Wallet</span>
    </button>
  );
}

