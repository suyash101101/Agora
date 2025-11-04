import React, { useState } from 'react';
import { useApiContext } from '../context/ApiContext';
import { DEFAULT_ENDPOINTS } from '../utils/constants';
import { Wifi, Save } from 'lucide-react';

export function Settings() {
  const { reconnect } = useApiContext();
  const [endpoint, setEndpoint] = useState(DEFAULT_ENDPOINTS.para1000);
  const [isSaving, setIsSaving] = useState(false);

  const handleSave = async () => {
    try {
      setIsSaving(true);
      await reconnect(endpoint);
      setIsSaving(false);
      alert('Endpoint updated successfully!');
    } catch (error) {
      setIsSaving(false);
      alert(`Failed to update endpoint: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  return (
    <div className="max-w-2xl mx-auto">
      <h1 className="text-3xl font-bold text-gray-900 mb-8">Settings</h1>
      
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <h2 className="text-xl font-semibold mb-4 flex items-center gap-2">
          <Wifi className="w-5 h-5" />
          Chain Connection
        </h2>
        
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              WebSocket Endpoint
            </label>
            <select
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
            >
              <option value={DEFAULT_ENDPOINTS.para1000}>Para 1000 (ws://localhost:9990)</option>
              <option value={DEFAULT_ENDPOINTS.para2000}>Para 2000 (ws://localhost:9991)</option>
              <option value={DEFAULT_ENDPOINTS.relay}>Relay (ws://localhost:9988)</option>
            </select>
            <input
              type="text"
              value={endpoint}
              onChange={(e) => setEndpoint(e.target.value)}
              placeholder="Or enter custom endpoint..."
              className="w-full mt-2 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
            />
          </div>
          
          <button
            onClick={handleSave}
            disabled={isSaving}
            className="flex items-center justify-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Save className="w-5 h-5" />
            <span>{isSaving ? 'Saving...' : 'Save Settings'}</span>
          </button>
        </div>
      </div>
    </div>
  );
}

