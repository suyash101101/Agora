import React from 'react';
import { useAccountContext } from '../../context/AccountContext';
import { Coins } from 'lucide-react';

export function BalanceDisplay() {
  const { balance, isLoading } = useAccountContext();

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 px-4 py-2 bg-gray-100 rounded-lg">
        <Coins className="w-4 h-4 animate-pulse" />
        <span className="text-sm text-gray-500">Loading...</span>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 px-4 py-2 bg-gray-100 rounded-lg">
      <Coins className="w-4 h-4" />
      <span className="text-sm font-medium">{balance} UNIT</span>
    </div>
  );
}

