import { useEffect, useState } from 'react';
import {
  walletBalance,
  nodeGetBalance,
  formatInvokeError,
} from '@/tauri';
import { Button, ErrorDisplay, Loading, AddressDisplay } from '@/components/ui';

interface DashboardProps {
  address: string;
  passphrase: string;
  onLock: () => void;
  onSend: () => void;
  onHistory: () => void;
  onDeriveAddress: () => void;
  nodeConnected: boolean;
  nodeInfo: { chainId: string; tipHeight: number; version: string } | null;
  onConnectNode: () => void;
  onDisconnectNode: () => void;
}

export function Dashboard({
  address,
  passphrase,
  onLock,
  onSend,
  onHistory,
  onDeriveAddress,
  nodeConnected,
  nodeInfo,
  onConnectNode,
  onDisconnectNode,
}: DashboardProps) {
  const [balance, setBalance] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchBalance = async () => {
    setLoading(true);
    setError(null);
    try {
      if (nodeConnected) {
        const response = await nodeGetBalance({ address });
        setBalance(response.balance);
      } else {
        const response = await walletBalance({ address });
        setBalance(response.balance);
      }
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    (async () => {
      await fetchBalance();
    })();
  }, [address, passphrase, nodeConnected, nodeInfo]);

  return (
    <div className="min-h-screen bg-slate-950 p-6">
      <div className="max-w-3xl mx-auto">
        <header className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-2xl font-bold text-white">Dashboard</h1>
            <p className="text-slate-400 text-sm">
              {nodeConnected ? 'live node' : 'offline wallet'}
            </p>
          </div>
          <div className="flex gap-2">
            {nodeConnected ? (
              <Button
                variant="secondary"
                onClick={onDisconnectNode}
                className="h-9 px-3 text-sm"
              >
                Disconnect node
              </Button>
            ) : (
              <Button
                variant="secondary"
                onClick={onConnectNode}
                className="h-9 px-3 text-sm"
              >
                Connect node
              </Button>
            )}
            <Button variant="secondary" onClick={onLock} className="h-9 px-3 text-sm">
              Lock
            </Button>
          </div>
        </header>

        {error && <ErrorDisplay message={error} />}

        {nodeConnected && nodeInfo && (
          <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-4 mb-6">
            <div className="flex items-center justify-between gap-4">
              <div>
                <p className="text-xs text-slate-400 uppercase tracking-wider">
                  Node connection
                </p>
                <p className="text-sm text-white font-mono">
                  {nodeInfo.chainId} • tip #{nodeInfo.tipHeight} • v{nodeInfo.version}
                </p>
              </div>
              <div className="w-2 h-2 rounded-full bg-emerald-500" />
            </div>
          </div>
        )}

        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-6">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-2">
              Address
            </h2>
            <div className="flex items-center gap-3">
              <AddressDisplay address={address} shorten={true} />
              <Button
                variant="secondary"
                className="h-8 w-8 px-0 text-xs"
                onClick={() => navigator.clipboard.writeText(address)}
                title="Copy address"
              >
                Copy
              </Button>
            </div>
          </div>

          <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-6">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-2">
              ATOLL Balance
            </h2>
            <div className="flex items-baseline gap-2">
              {loading ? (
                <Loading className="h-6 w-6" />
              ) : (
                <span className="text-4xl font-bold text-indigo-400">
                  {balance === null ? '–' : balance}
                </span>
              )}
              <span className="text-slate-500 text-sm">ATOLL</span>
            </div>
            {!nodeConnected && balance === 0 && (
              <p className="mt-2 text-xs text-slate-500">
                Offline ChainState — balance is 0 until a live node is connected.
              </p>
            )}
            <Button
              variant="secondary"
              onClick={fetchBalance}
              className="mt-4 w-full text-xs"
              disabled={loading}
            >
              Refresh balance
            </Button>
          </div>
        </div>

        {!nodeConnected && (
          <div className="mt-6 bg-slate-900/50 border border-slate-700/50 rounded-xl p-6">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-3">
              Connect to node
            </h2>
            <p className="text-xs text-slate-500 mb-3">
              Connect to a running atoll-node to see real balances and submit
              transactions. The node must be running on the address below.
            </p>
            <div className="space-y-3">
              <div className="flex items-center gap-2">
                <span className="text-xs text-slate-400">Node URL</span>
                <input
                  type="text"
                  value="http://localhost:8080"
                  readOnly
                  className="flex-1 block w-full rounded-md border border-slate-600 bg-slate-900 px-3 py-2 text-xs text-slate-100 font-mono"
                />
              </div>
              <Button variant="primary" onClick={onConnectNode} className="w-full">
                Connect
              </Button>
            </div>
          </div>
        )}

        <div className="mt-8 grid grid-cols-1 md:grid-cols-3 gap-4">
          <Button variant="secondary" onClick={onSend} className="w-full">
            Send ATOLL
          </Button>
          <Button variant="secondary" onClick={onHistory} className="w-full">
            History
          </Button>
          <Button variant="secondary" onClick={onDeriveAddress} className="w-full">
            Derive Address
          </Button>
        </div>

        <footer className="mt-10 pt-6 border-t border-slate-800 text-xs text-slate-500">
          Keystore: ~/.atoll/keystore.json (shared with the CLI wallet)
        </footer>
      </div>
    </div>
  );
}
