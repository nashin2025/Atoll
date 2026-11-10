import { useState, useCallback } from 'react';
import { Welcome } from '@/views/Welcome';
import { Dashboard } from '@/views/Dashboard';
import { Send } from '@/views/Send';
import { History } from '@/views/History';
import { DeriveAddress } from '@/views/DeriveAddress';
import { Staking } from '@/views/Staking';
import {
  nodeStart,
  nodeStop,
  nodeStatus,
  nodeMine,
  nodeAutoMine,
  nodeAddPeer,
  NodeStartResponse,
  NodeStatusResponse,
  NodeMineResponse,
  NodeAutoMineResponse,
  NodeAddPeerResponse,
} from '@/tauri';
import { NavLink, AddressDisplay } from '@/components/ui';

type View = 'welcome' | 'dashboard' | 'send' | 'history' | 'derive-address' | 'staking';

interface AppState {
  view: View;
  addresses: string[];
  selectedAddress: string | null;
  passphrase: string;
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
}

const VIEWS: { id: View; label: string; icon: string }[] = [
  { id: 'dashboard', label: 'Dashboard', icon: '🏠' },
  { id: 'send', label: 'Send', icon: '📤' },
  { id: 'history', label: 'History', icon: '📜' },
  { id: 'derive-address', label: 'Derive Address', icon: '🔗' },
  { id: 'staking', label: 'Staking', icon: '📊' },
];

export default function App() {
  const [state, setState] = useState<AppState>({
    view: 'welcome',
    addresses: [],
    selectedAddress: null,
    passphrase: '',
    nodeConnected: false,
    nodeInfo: null,
  });

  const handleUnlocked = useCallback(
    (addresses: string[], passphrase: string) => {
      setState((prev) => ({
        ...prev,
        view: 'dashboard',
        addresses,
        selectedAddress: addresses[0] ?? null,
        passphrase,
      }));
    },
    [],
  );

  const handleLock = useCallback(() => {
    setState((prev) => ({
      ...prev,
      view: 'welcome',
      addresses: [],
      selectedAddress: null,
      passphrase: '',
    }));
  }, []);

  const handleStartNode = useCallback(async () => {
    try {
      const result: NodeStartResponse = await nodeStart({});
      setState((prev) => ({
        ...prev,
        nodeConnected: true,
        nodeInfo: {
          chainId: result.chain_id,
          tipHeight: result.tip_height,
          finalizedHeight: 0,
          blockCount: 1,
          peerCount: 0,
          mempoolSize: 0,
          autoMine: true,
          lastBlockTxCount: 0,
          genesisHash: '',
          syncing: false,
          version: result.version,
        },
      }));
    } catch (err) {
      console.error('Node start failed:', err);
    }
  }, []);

  const handleStopNode = useCallback(async () => {
    try {
      await nodeStop({});
    } catch {
      // Ignore errors on stop
    }
    setState((prev) => ({
      ...prev,
      nodeConnected: false,
      nodeInfo: null,
    }));
  }, []);

  const handleRefreshNodeStatus = useCallback(async () => {
    try {
      const result: NodeStatusResponse = await nodeStatus({});
      setState((prev) => ({
        ...prev,
        nodeConnected: result.running,
        nodeInfo: {
          chainId: result.chain_id,
          tipHeight: result.tip_height,
          finalizedHeight: result.finalized_height,
          blockCount: result.block_count,
          peerCount: result.peer_count,
          mempoolSize: result.mempool_size,
          autoMine: result.auto_mine,
          lastBlockTxCount: result.last_block_tx_count,
          genesisHash: result.genesis_hash,
          syncing: result.syncing,
          version: result.version,
        },
      }));
    } catch (err) {
      console.error('Node status refresh failed:', err);
    }
  }, []);

  const clearPassphrase = useCallback(() => {
    setState((prev) => ({ ...prev, passphrase: '' }));
  }, []);

  const handleMine = useCallback(async () => {
    try {
      const result: NodeMineResponse = await nodeMine({});
      setState((prev) => ({
        ...prev,
        nodeInfo: prev.nodeInfo
          ? {
              ...prev.nodeInfo,
              tipHeight: result.height,
              blockCount: result.height + 1,
            }
          : prev.nodeInfo,
      }));
      await handleRefreshNodeStatus();
      return result.txs_included;
    } catch (err) {
      console.error('Mine failed:', err);
      return null;
    }
  }, []);

  const handleAutoMine = useCallback(async (enabled: boolean) => {
    try {
      const result: NodeAutoMineResponse = await nodeAutoMine({ enabled });
      setState((prev) => ({
        ...prev,
        nodeInfo: prev.nodeInfo
          ? { ...prev.nodeInfo, autoMine: result.enabled }
          : prev.nodeInfo,
      }));
    } catch (err) {
      console.error('Auto-mine toggle failed:', err);
    }
  }, []);

  const handleAddPeer = useCallback(
    async (address: string, nodeId?: string) => {
      try {
        const result: NodeAddPeerResponse = await nodeAddPeer({
          address,
          node_id: nodeId,
        });
        if (result.added) {
          await handleRefreshNodeStatus();
        }
      } catch (err) {
        console.error('Add peer failed:', err);
      }
    },
    []
  );

  const selectedAddress = state.selectedAddress;
  const passphrase = state.passphrase;
  const activeView = state.view;

  // Welcome view — full screen, no sidebar
  if (state.view === 'welcome') {
    return (
      <div className="min-h-screen bg-slate-900">
        <Welcome onUnlocked={handleUnlocked} />
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-slate-950">
      {/* Sidebar */}
      <aside className="fixed left-0 top-0 h-full w-36 bg-slate-900/80 border-r border-slate-800/50 backdrop-blur-sm z-50 flex flex-col">
        <div className="flex items-center justify-center h-16 px-1">
          <span className="text-xl">🪙</span>
        </div>
        <nav className="flex-1 px-1 py-4 space-y-1">
          {VIEWS.map((v) => (
            <NavLink
              key={v.id}
              active={activeView === v.id}
              onClick={() =>
                setState((prev) => ({ ...prev, view: v.id }))
              }
              icon={<span className="text-xs">{v.icon}</span>}
              title={v.label}
            >
              <span className="whitespace-nowrap">{v.label}</span>
            </NavLink>
          ))}
        </nav>
        <div className="px-2 py-4">
          <NavLink
            active={false}
            onClick={handleLock}
            icon={<span className="text-xs">🔒</span>}
          >
            Lock
          </NavLink>
        </div>
      </aside>

      {/* Main content */}
      <div className="ml-36 p-6">
        <header className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-xl font-bold text-white tracking-tight">
              {VIEWS.find((v) => v.id === activeView)?.label}
            </h1>
            {selectedAddress && (
              <div className="mt-1 flex items-center gap-2 text-xs text-slate-400">
                <span className="w-2 h-2 rounded-full bg-emerald-400" />
                <span>Wallet unlocked</span>
              </div>
            )}
          </div>
          <div className="flex items-center gap-3">
            <AddressDisplay
              address={selectedAddress ?? ''}
              shorten={true}
              onCopy={() => navigator.clipboard.writeText(selectedAddress ?? '')}
            />
            <div className="flex items-center gap-2">
              <span className={`w-2 h-2 rounded-full ${
                state.nodeConnected ? 'bg-emerald-400' : 'bg-slate-500'
              }`} />
              <span className="text-xs text-slate-400">
                {state.nodeConnected ? 'Node online' : 'Node offline'}
              </span>
            </div>
          </div>
        </header>

        {state.view === 'dashboard' && selectedAddress && (
          <Dashboard
            address={selectedAddress}
            passphrase={passphrase}
            onLock={handleLock}
            onSend={() => setState((prev) => ({ ...prev, view: 'send' }))}
            onHistory={() => setState((prev) => ({ ...prev, view: 'history' }))}
            onDeriveAddress={() => setState((prev) => ({ ...prev, view: 'derive-address' }))}
            nodeConnected={state.nodeConnected}
            nodeInfo={state.nodeInfo}
            onStartNode={handleStartNode}
            onStopNode={handleStopNode}
            onRefreshNodeStatus={handleRefreshNodeStatus}
            onMine={handleMine}
            onAutoMine={handleAutoMine}
            onAddPeer={handleAddPeer}
          />
        )}

        {state.view === 'send' && selectedAddress && (
          <Send
            from={selectedAddress}
            passphrase={passphrase}
            onBack={handleLock}
            onSendComplete={clearPassphrase}
            nodeConnected={state.nodeConnected}
          />
        )}

        {state.view === 'history' && (
          <History
            onBack={() => setState((prev) => ({ ...prev, view: 'dashboard' }))}
            nodeConnected={state.nodeConnected}
          />
        )}

        {state.view === 'derive-address' && (
          <DeriveAddress onBack={() => setState((prev) => ({ ...prev, view: 'dashboard' }))} />
        )}

        {state.view === 'staking' && selectedAddress && (
          <Staking
            address={selectedAddress}
            passphrase={passphrase}
            onBack={() => setState((prev) => ({ ...prev, view: 'dashboard' }))}
          />
        )}
      </div>
    </div>
  );
}
