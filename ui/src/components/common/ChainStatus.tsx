import React from 'react';
import { useApiContext } from '../../context/ApiContext';
import { Wifi, WifiOff, Loader } from 'lucide-react';

export function ChainStatus() {
  const { isConnected, isLoading, error } = useApiContext();

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 px-3 py-1.5 bg-gray-100 rounded-lg">
        <Loader className="w-4 h-4 animate-spin" />
        <span className="text-xs text-gray-600">Connecting...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center gap-2 px-3 py-1.5 bg-red-100 rounded-lg">
        <WifiOff className="w-4 h-4 text-red-600" />
        <span className="text-xs text-red-600">Connection Error</span>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 px-3 py-1.5 bg-green-100 rounded-lg">
      <Wifi className="w-4 h-4 text-green-600" />
      <span className="text-xs text-green-600">Connected</span>
    </div>
  );
}

