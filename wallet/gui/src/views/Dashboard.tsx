import { useEffect, useState } from 'react';
import {
  walletBalance,
  nodeGetBalance,
  nodeMine,
  nodeAutoMine,
  nodeAddPeer,
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
  nodeInfo: {
    chainId: number;
    tipHeight: number;
    finalizedHeight: number;
    blockCount: number;
    peerCount: number;
    mempoolSize: number;
    autoMine: boolean;
    syncing: boolean;
    version: string;
  } | null;
  onStartNode: () => void;
  onStopNode: () => void;
  onRefreshNodeStatus: () => void;
  onMine: () => Promise<void>;
  onAutoMine: (enabled: boolean) => Promise<void>;
  onAddPeer: (address: string, nodeId?: string) => Promise<void>;
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
  onStartNode,
  onStopNode,
  onRefreshNodeStatus,
}: DashboardProps) {
  const [balance, setBalance] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mineLoading, setMineLoading] = useState(false);
  const [autoMineLoading, setAutoMineLoading] = useState(false);
  const [addPeerLoading, setAddPeerLoading] = useState(false);
  const [addPeerError, setAddPeerError] = useState<string | null>(null);
  const [peerAddress, setPeerAddress] = useState('127.0.0.1:30333');
  const [peerNodeId, setPeerNodeId] = useState('');

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

  const handleMine = async () => {
    setMineLoading(true);
    try {
      await nodeMine({});
      await fetchBalance();
      await refreshNodeStatus();
    } catch (err) {
      setAddPeerError(formatInvokeError(err));
    } finally {
      setMineLoading(false);
    }
  };

  const handleAutoMine = async (enabled: boolean) => {
    setAutoMineLoading(true);
    try {
      await nodeAutoMine({ enabled });
      await refreshNodeStatus();
    } catch (err) {
      setAddPeerError(formatInvokeError(err));
    } finally {
      setAutoMineLoading(false);
    }
  };

  const handleAddPeer = async () => {
    setAddPeerLoading(true);
    setAddPeerError(null);
    try {
      await nodeAddPeer({ address: peerAddress, node_id: peerNodeId || undefined });
      setAddPeerError(null);
    } catch (err) {
      setAddPeerError(formatInvokeError(err));
    } finally {
      setAddPeerLoading(false);
    }
  };

  const refreshNodeStatus = async () => {
    try {
      await onRefreshNodeStatus();
    } catch {
      // ignore refresh errors
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
              {nodeConnected ? 'embedded node running' : 'offline wallet'}
            </p>
          </div>
          <div className="flex gap-2">
            {nodeConnected ? (
              <Button
                variant="secondary"
                onClick={onStopNode}
                className="h-9 px-3 text-sm"
              >
                Stop node
              </Button>
            ) : (
              <Button
                variant="secondary"
                onClick={onStartNode}
                className="h-9 px-3 text-sm"
              >
                Start node
              </Button>
            )}
            <Button
              variant="secondary"
              onClick={onRefreshNodeStatus}
              className="h-9 px-3 text-sm"
            >
              Refresh
            </Button>
            <Button variant="secondary" onClick={onLock} className="h-9 px-3 text-sm">
              Lock
            </Button>
          </div>
        </header>

        {error && <ErrorDisplay message={error} />}

        {nodeConnected && nodeInfo && (
          <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-4 mb-6">
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div className="bg-slate-800/50 rounded-lg p-3">
                <p className="text-xs text-slate-400 uppercase tracking-wider">
                  Chain ID
                </p>
                <p className="text-sm text-white font-mono">
                  {nodeInfo.chainId}
                </p>
              </div>
              <div className="bg-slate-800/50 rounded-lg p-3">
                <p className="text-xs text-slate-400 uppercase tracking-wider">
                  Tip height
                </p>
                <p className="text-sm text-white font-mono">
                  #{nodeInfo.tipHeight}
                </p>
              </div>
              <div className="bg-slate-800/50 rounded-lg p-3">
                <p className="text-xs text-slate-400 uppercase tracking-wider">
                  Peers
                </p>
                <p className="text-sm text-white font-mono">
                  {nodeInfo.peerCount}
                </p>
              </div>
              <div className="bg-slate-800/50 rounded-lg p-3">
                <p className="text-xs text-slate-400 uppercase tracking-wider">
                  Mempool
                </p>
                <p className="text-sm text-white font-mono">
                  {nodeInfo.mempoolSize}
                </p>
              </div>
              <div className="bg-slate-800/50 rounded-lg p-3">
                <p className="text-xs text-slate-400 uppercase tracking-wider">
                  Auto-mine
                </p>
                <p className={`text-sm font-mono ${
                  nodeInfo.autoMine
                    ? 'text-emerald-400'
                    : 'text-slate-400'
                }`}>
                  {nodeInfo.autoMine ? 'on' : 'off'}
                </p>
              </div>
              <div className="bg-slate-800/50 rounded-lg p-3">
                <p className="text-xs text-slate-400 uppercase tracking-wider">
                  Syncing
                </p>
                <p className={`text-sm font-mono ${
                  nodeInfo.syncing
                    ? 'text-amber-400'
                    : 'text-emerald-400'
                }`}>
                  {nodeInfo.syncing ? 'yes' : 'no'}
                </p>
              </div>
            </div>
          </div>
        )}

        {nodeConnected && nodeInfo && (
          <div className="bg-slate-900/50 border border-slate-700/50 rounded-xl p-6">
            <h2 className="text-sm font-semibold text-slate-400 uppercase tracking-wider mb-4">
              Node controls
            </h2>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
              <Button
                variant={nodeInfo.autoMine ? 'primary' : 'secondary'}
                onClick={handleMine}
                className="w-full"
                disabled={mineLoading}
              >
                {mineLoading ? (
                  <span className="flex items-center gap-2">
                    <Loading className="h-4 w-4" />
                    Mining...
                  </span>
                ) : (
                  'Mine block'
                )}
              </Button>
              <Button
                variant={nodeInfo.autoMine ? 'secondary' : 'secondary'}
                onClick={() => handleAutoMine(!nodeInfo.autoMine)}
                className="w-full"
                disabled={autoMineLoading}
              >
                {autoMineLoading ? (
                  <span className="flex items-center gap-2">
                    <Loading className="h-4 w-4" />
                  </span>
                ) : (
                  nodeInfo.autoMine ? 'Disable auto-mine' : 'Enable auto-mine'
                )}
              </Button>
              <Button
                variant="secondary"
                onClick={onRefreshNodeStatus}
                className="w-full"
              >
                Refresh status
              </Button>
            </div>

            {addPeerError && (
              <div className="mb-4 p-3 rounded bg-red-500/10 text-red-400 text-xs">
                {addPeerError}
              </div>
            )}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="text-xs text-slate-400 uppercase tracking-wider mb-2 block">
                  Peer address (host:port)
                </label>
                <input
                  type="text"
                  value={peerAddress}
                  onChange={(e) => setPeerAddress(e.target.value)}
                  placeholder="127.0.0.1:30333"
                  className="w-full bg-slate-800 border border-slate-700 rounded px-3 py-2 text-sm text-white"
                  disabled={addPeerLoading}
                />
              </div>
              <div>
                <label className="text-xs text-slate-400 uppercase tracking-wider mb-2 block">
                  Node ID (hex, optional)
                </label>
                <input
                  type="text"
                  value={peerNodeId}
                  onChange={(e) => setPeerNodeId(e.target.value)}
                  placeholder="64 hex chars"
                  className="w-full bg-slate-800 border border-slate-700 rounded px-3 py-2 text-sm text-white"
                  disabled={addPeerLoading}
                />
              </div>
              <Button
                variant="primary"
                onClick={handleAddPeer}
                className="w-full"
                disabled={addPeerLoading}
              >
                {addPeerLoading ? 'Adding...' : 'Add peer'}
              </Button>
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
                Start the embedded node to see real balances.
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
              Embedded node
            </h2>
            <p className="text-xs text-slate-500 mb-3">
              The GUI wallet runs a full Atoll node in-process. Start it to
              validate the chain, see real balances, and submit transactions
              directly — no separate node process required.
            </p>
            <Button variant="primary" onClick={onStartNode} className="w-full">
              Start embedded node
            </Button>
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
