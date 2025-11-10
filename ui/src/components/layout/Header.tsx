import { ConnectButton } from '../common/ConnectButton';
import { BalanceDisplay } from '../common/BalanceDisplay';
import { ChainStatus } from '../common/ChainStatus';

export function Header() {
  return (
    <header className="bg-white border-b border-gray-200 shadow-sm">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          <div className="flex items-center gap-3">
            <img src="/agora-logo.svg" alt="Agora Logo" className="w-8 h-8" />
            <h1 className="text-xl font-bold text-gray-900">Agora Parachain</h1>
          </div>
          <div className="flex items-center gap-4">
            <ChainStatus />
            <BalanceDisplay />
            <ConnectButton />
          </div>
        </div>
      </div>
    </header>
  );
}

