import React, { useState } from 'react';
import { useAccountContext } from '../../context/AccountContext';
import { formatAddress } from '../../utils/formatters';

interface AccountSelectorProps {
  onSelect?: (address: string) => void;
}

export function AccountSelector({ onSelect }: AccountSelectorProps) {
  const { account, connectExtension, connectManual } = useAccountContext();
  const [manualAddress, setManualAddress] = useState('');
  const [showManual, setShowManual] = useState(false);

  const handleExtensionConnect = async () => {
    try {
      await connectExtension();
    } catch (error) {
      alert(`Failed to connect: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const handleManualConnect = () => {
    if (manualAddress.trim()) {
      connectManual(manualAddress.trim());
      setManualAddress('');
      setShowManual(false);
      if (onSelect) {
        onSelect(manualAddress.trim());
      }
    }
  };

  if (account) {
    return (
      <div className="flex items-center gap-2">
        <div className="px-3 py-1.5 bg-gray-100 rounded-lg">
          <span className="text-sm font-medium">{formatAddress(account.address)}</span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <button
        onClick={handleExtensionConnect}
        className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors"
      >
        Connect Extension
      </button>
      <button
        onClick={() => setShowManual(!showManual)}
        className="px-4 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 transition-colors text-sm"
      >
        {showManual ? 'Cancel' : 'Enter Address Manually'}
      </button>
      {showManual && (
        <div className="flex gap-2">
          <input
            type="text"
            value={manualAddress}
            onChange={(e) => setManualAddress(e.target.value)}
            placeholder="Enter address..."
            className="flex-1 px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
          />
          <button
            onClick={handleManualConnect}
            disabled={!manualAddress.trim()}
            className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Connect
          </button>
        </div>
      )}
    </div>
  );
}

