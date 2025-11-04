import React, { useState } from 'react';
import { useApiContext } from '../../context/ApiContext';
import { useAccountContext } from '../../context/AccountContext';
import { useWorkers } from '../../hooks/useWorkers';
import { MIN_WORKER_STAKE } from '../../utils/constants';
import { formatBalance, formatAddress } from '../../utils/formatters';
import { parseBalance } from '../../utils/formatters';
import { validateStake } from '../../utils/helpers';
import { UserPlus, UserMinus, Coins, Award, Loader } from 'lucide-react';

export function WorkerDashboard() {
  const { api } = useApiContext();
  const { account, getSigner } = useAccountContext();
  const { workers, loadWorker } = useWorkers(api);
  const [stake, setStake] = useState('');
  const [isRegistering, setIsRegistering] = useState(false);
  const [isUnregistering, setIsUnregistering] = useState(false);

  const worker = account ? workers.get(account.address) : null;

  React.useEffect(() => {
    if (account && api) {
      loadWorker(account.address);
    }
  }, [account, api, loadWorker]);

  const handleRegister = async () => {
    if (!api || !account) return;

    const stakeBN = parseBalance(stake);
    const validation = validateStake(BigInt(stakeBN.toString()), MIN_WORKER_STAKE);
    if (!validation.valid) {
      alert(validation.error);
      return;
    }

    try {
      setIsRegistering(true);
      const signer = await getSigner(account.address);
      const tx = api.tx.agora.registerWorker(stakeBN.toString());
      
      await tx.signAndSend(account.address, { signer }, ({ status }) => {
        if (status.isInBlock) {
          loadWorker(account.address);
          setStake('');
          setIsRegistering(false);
          alert('Worker registered successfully!');
        } else if (status.isFinalized) {
          setIsRegistering(false);
        }
      });
    } catch (error) {
      console.error('Failed to register worker:', error);
      setIsRegistering(false);
      alert(`Failed to register: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  const handleUnregister = async () => {
    if (!api || !account) return;

    if (!confirm('Are you sure you want to unregister as a worker?')) {
      return;
    }

    try {
      setIsUnregistering(true);
      const signer = await getSigner(account.address);
      const tx = api.tx.agora.unregisterWorker();
      
      await tx.signAndSend(account.address, { signer }, ({ status }) => {
        if (status.isInBlock) {
          loadWorker(account.address);
          setIsUnregistering(false);
          alert('Worker unregistered successfully!');
        } else if (status.isFinalized) {
          setIsUnregistering(false);
        }
      });
    } catch (error) {
      console.error('Failed to unregister worker:', error);
      setIsUnregistering(false);
      alert(`Failed to unregister: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  };

  if (!account) {
    return (
      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <p className="text-gray-500">Please connect your wallet to view worker dashboard</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {worker ? (
        <div className="bg-white rounded-lg border border-gray-200 p-6">
          <h2 className="text-xl font-semibold mb-4">Worker Status</h2>
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <UserPlus className="w-5 h-5 text-green-600" />
              <span className="font-medium">Registered Worker</span>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div>
                <p className="text-sm text-gray-500">Stake</p>
                <p className="font-medium flex items-center gap-2">
                  <Coins className="w-4 h-4" />
                  {formatBalance(worker.stake.toString())} UNIT
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500">Reputation</p>
                <p className="font-medium flex items-center gap-2">
                  <Award className="w-4 h-4" />
                  {worker.reputation}/1000
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500">Status</p>
                <p className="font-medium">
                  {worker.isActive ? 'Active' : 'Inactive'}
                </p>
              </div>
              <div>
                <p className="text-sm text-gray-500">Registered At</p>
                <p className="font-medium">Block {worker.registeredAt.toString()}</p>
              </div>
            </div>
            <button
              onClick={handleUnregister}
              disabled={isUnregistering}
              className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isUnregistering ? (
                <>
                  <Loader className="w-5 h-5 animate-spin" />
                  <span>Unregistering...</span>
                </>
              ) : (
                <>
                  <UserMinus className="w-5 h-5" />
                  <span>Unregister Worker</span>
                </>
              )}
            </button>
          </div>
        </div>
      ) : (
        <div className="bg-white rounded-lg border border-gray-200 p-6">
          <h2 className="text-xl font-semibold mb-4">Register as Worker</h2>
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Stake Amount (UNIT)
              </label>
              <input
                type="text"
                value={stake}
                onChange={(e) => setStake(e.target.value)}
                placeholder={`Minimum: ${MIN_WORKER_STAKE.toString()}`}
                className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
              />
            </div>
            <button
              onClick={handleRegister}
              disabled={isRegistering || !stake.trim()}
              className="w-full flex items-center justify-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isRegistering ? (
                <>
                  <Loader className="w-5 h-5 animate-spin" />
                  <span>Registering...</span>
                </>
              ) : (
                <>
                  <UserPlus className="w-5 h-5" />
                  <span>Register Worker</span>
                </>
              )}
            </button>
          </div>
        </div>
      )}

      <div className="bg-white rounded-lg border border-gray-200 p-6">
        <h2 className="text-xl font-semibold mb-4">All Workers</h2>
        <div className="space-y-3">
          {Array.from(workers.entries()).map(([address, workerInfo]) => (
            <div key={address} className="bg-gray-50 p-4 rounded-lg">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium">{formatAddress(address)}</p>
                  <p className="text-sm text-gray-500">
                    Reputation: {workerInfo.reputation}/1000
                  </p>
                </div>
                <div className="text-right">
                  <p className="font-medium">{formatBalance(workerInfo.stake.toString())} UNIT</p>
                  <p className="text-xs text-gray-500">
                    {workerInfo.isActive ? 'Active' : 'Inactive'}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

