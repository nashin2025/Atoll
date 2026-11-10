import { useEffect, useState } from "react";
import { walletBalance, nodeGetBalance, formatInvokeError } from "@/tauri";
import {
  Button,
  ErrorDisplay,
  AddressDisplay,
  Card,
  CardHeader,
  StatCard,
  Badge,
  Skeleton,
  SuccessDisplay,
} from "@/components/ui";

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
    lastBlockTxCount: number;
    genesisHash: string;
    syncing: boolean;
    version: string;
  } | null;
  onStartNode: () => void;
  onStopNode: () => void;
  onRefreshNodeStatus: () => void;
  onMine: () => Promise<number | null>;
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
  onMine,
  onAutoMine,
  onAddPeer,
}: DashboardProps) {
  const [balance, setBalance] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [mineLoading, setMineLoading] = useState(false);
  const [autoMineLoading, setAutoMineLoading] = useState(false);
  const [addPeerLoading, setAddPeerLoading] = useState(false);
  const [addPeerError, setAddPeerError] = useState<string | null>(null);
  const [peerAddress, setPeerAddress] = useState("127.0.0.1:30333");
  const [peerNodeId, setPeerNodeId] = useState("");

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
    setError(null);
    try {
      await onMine();
      await fetchBalance();
      await onRefreshNodeStatus();
      setSuccess("Block mined successfully");
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setMineLoading(false);
    }
  };

  const handleAutoMine = async (enabled: boolean) => {
    setAutoMineLoading(true);
    setError(null);
    try {
      await onAutoMine(enabled);
      await onRefreshNodeStatus();
      setSuccess(`Auto-mine ${enabled ? "enabled" : "disabled"}`);
    } catch (err) {
      setError(formatInvokeError(err));
    } finally {
      setAutoMineLoading(false);
    }
  };

  const handleAddPeer = async () => {
    setAddPeerLoading(true);
    setAddPeerError(null);
    try {
      await onAddPeer(peerAddress, peerNodeId || undefined);
      setSuccess("Peer added successfully");
      setPeerAddress("");
      setPeerNodeId("");
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
      <div className="max-w-5xl mx-auto">
        <header className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-2xl font-bold text-white tracking-tight">Dashboard</h1>
            <p className="text-slate-400 text-sm">
              {nodeConnected ? "Embedded node running" : "Offline wallet"}
            </p>
          </div>
          <div className="flex gap-3">
            {nodeConnected ? (
              <Button variant="danger" onClick={onStopNode} size="sm">
                Stop node
              </Button>
            ) : (
              <Button variant="primary" onClick={onStartNode} size="sm">
                Start node
              </Button>
            )}
            <Button variant="secondary" onClick={onRefreshNodeStatus} size="sm">
              Refresh
            </Button>
            <Button variant="secondary" onClick={onLock} size="sm">
              Lock
            </Button>
          </div>
        </header>

        {error && <ErrorDisplay message={error} />}
        {success && <SuccessDisplay message={success} />}

        {/* Status overview cards */}
        {nodeConnected && nodeInfo && (
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
            <StatCard title="Tip height" value={`#${nodeInfo.tipHeight}`} variant="default" />
            <StatCard
              title="Block count"
              value={nodeInfo.blockCount.toLocaleString()}
              variant="default"
            />
            <StatCard
              title="Block TXs"
              value={nodeInfo.lastBlockTxCount}
              variant={nodeInfo.lastBlockTxCount === 0 ? "warning" : "success"}
            />
            <StatCard
              title="Peers"
              value={nodeInfo.peerCount}
              variant={nodeInfo.peerCount > 0 ? "success" : "warning"}
            />
            <StatCard title="Mempool" value={nodeInfo.mempoolSize} variant="default" />
            <StatCard
              title="Balance"
              value={balance === null ? "–" : balance}
              variant="success"
              change="ATOLL"
            />
          </div>
        )}

        {/* Node status panel */}
        {nodeConnected && nodeInfo && (
          <Card>
            <CardHeader
              title="Node status"
              description="Live metrics from the embedded Atoll node"
            />
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
              <div className="bg-slate-800/50 rounded-lg p-4">
                <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">
                  Chain ID
                </p>
                <p className="text-sm font-mono text-white">{nodeInfo.chainId}</p>
              </div>
              <div className="bg-slate-800/50 rounded-lg p-4">
                <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">
                  Block count
                </p>
                <p className="text-sm font-mono text-white">{nodeInfo.blockCount}</p>
              </div>
              <div className="bg-slate-800/50 rounded-lg p-4">
                <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">
                  Finalized height
                </p>
                <p className="text-sm font-mono text-white">#{nodeInfo.finalizedHeight}</p>
              </div>
              <div className="bg-slate-800/50 rounded-lg p-4">
                <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">
                  Syncing
                </p>
                <p
                  className={`text-sm font-mono ${
                    nodeInfo.syncing ? "text-amber-400" : "text-emerald-400"
                  }`}
                >
                  {nodeInfo.syncing ? "Yes" : "No"}
                </p>
              </div>
            </div>
            <div className="flex items-center justify-between pt-4 border-t border-slate-700/50">
              <div className="flex items-center gap-2">
                <Badge variant={nodeInfo.autoMine ? "success" : "warning"}>
                  {nodeInfo.autoMine ? "Auto-mine on" : "Auto-mine off"}
                </Badge>
              </div>
              <div className="text-xs text-slate-500">v{nodeInfo.version}</div>
            </div>
            <div className="flex items-center justify-between pt-3 border-t border-slate-700/50">
              <div className="flex items-center gap-2">
                <span className="text-xs font-semibold text-slate-400 uppercase tracking-wider">
                  Genesis hash
                </span>
              </div>
              <div className="flex-1 text-right">
                <code className="text-xs font-mono text-slate-300 truncate max-w-xs block">
                  {nodeInfo.genesisHash.slice(0, 12)}…{nodeInfo.genesisHash.slice(-12)}
                </code>
              </div>
            </div>
          </Card>
        )}

        {/* Node controls */}
        {nodeConnected && nodeInfo && (
          <Card>
            <CardHeader title="Node controls" />
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
              <Button
                variant={nodeInfo.autoMine ? "primary" : "secondary"}
                onClick={handleMine}
                className="w-full"
                disabled={mineLoading}
              >
                {mineLoading ? (
                  <span className="flex items-center gap-2">
                    <Skeleton className="h-5 w-5 border-none rounded-full border-2 border-t-transparent border-indigo-500 animate-spin" />
                    Mining…
                  </span>
                ) : (
                  "Mine block"
                )}
              </Button>
              <Button
                variant={nodeInfo.autoMine ? "secondary" : "primary"}
                onClick={() => handleAutoMine(!nodeInfo.autoMine)}
                className="w-full"
                disabled={autoMineLoading}
              >
                {autoMineLoading ? (
                  <Skeleton className="h-5 w-5 border-none rounded-full border-2 border-t-transparent border-indigo-500 animate-spin" />
                ) : nodeInfo.autoMine ? (
                  "Disable auto-mine"
                ) : (
                  "Enable auto-mine"
                )}
              </Button>
              <Button variant="secondary" onClick={refreshNodeStatus} className="w-full">
                Refresh status
              </Button>
            </div>

            {addPeerError && (
              <div className="mb-4 p-3 rounded-md border border-rose-500/50 bg-rose-900/20 text-rose-400 text-xs">
                {addPeerError}
              </div>
            )}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2 block">
                  Peer address (host:port)
                </label>
                <input
                  type="text"
                  value={peerAddress}
                  onChange={(e) => setPeerAddress(e.target.value)}
                  placeholder="127.0.0.1:30333"
                  className="w-full rounded-md border border-slate-600 bg-slate-900 text-slate-100 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 focus:outline-none px-3 py-2 text-sm transition-colors"
                  disabled={addPeerLoading}
                />
              </div>
              <div>
                <label className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2 block">
                  Node ID (hex, optional)
                </label>
                <input
                  type="text"
                  value={peerNodeId}
                  onChange={(e) => setPeerNodeId(e.target.value)}
                  placeholder="64 hex chars"
                  className="w-full rounded-md border border-slate-600 bg-slate-900 text-slate-100 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 focus:outline-none px-3 py-2 text-sm transition-colors"
                  disabled={addPeerLoading}
                />
              </div>
              <Button
                variant="primary"
                onClick={handleAddPeer}
                className="w-full"
                disabled={addPeerLoading}
              >
                {addPeerLoading ? "Adding…" : "Add peer"}
              </Button>
            </div>
          </Card>
        )}

        {/* Address + Balance */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
          <Card>
            <CardHeader title="Your address" description="Share this to receive ATOLL" />
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
          </Card>

          <Card>
            <CardHeader title="ATOLL Balance" />
            <div className="flex items-baseline gap-2">
              {loading ? (
                <Skeleton className="h-8 w-24" />
              ) : (
                <span className="text-4xl font-bold text-indigo-400 font-mono">
                  {balance === null ? "–" : balance}
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
          </Card>
        </div>

        {!nodeConnected && (
          <Card className="mt-6">
            <CardHeader title="Embedded node" />
            <p className="text-xs text-slate-500 mb-4">
              The GUI wallet runs a full Atoll node in-process. Start it to validate the chain, see
              real balances, and submit transactions directly — no separate node process required.
            </p>
            <Button variant="primary" onClick={onStartNode} className="w-full">
              Start embedded node
            </Button>
          </Card>
        )}

        {/* Quick actions */}
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

        <footer className="mt-12 pt-6 border-t border-slate-800 text-xs text-slate-500">
          Keystore: ~/.atoll/keystore.json (shared with the CLI wallet)
        </footer>
      </div>
    </div>
  );
}
