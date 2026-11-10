import { useState, useCallback } from 'react';
import { Welcome } from '@/views/Welcome';
import { Dashboard } from '@/views/Dashboard';
import { Send } from '@/views/Send';
import { History } from '@/views/History';
import { DeriveAddress } from '@/views/DeriveAddress';
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
import { Staking } from '@/views/Staking';

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
    syncing: boolean;
    version: string;
  } | null;
}

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
          syncing: result.syncing,
          version: result.version,
        },
      }));
    } catch (err) {
      console.error('Node status refresh failed:', err);
    }
  }, []);

  /** Clears the passphrase from React state once an operation is finished. */
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
      // Refresh to get updated mempool size and other state
      await handleRefreshNodeStatus();
    } catch (err) {
      console.error('Mine failed:', err);
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

  return (
    <div className="min-h-screen bg-slate-950">
      {state.view === 'welcome' && (
        <Welcome onUnlocked={handleUnlocked} />
      )}

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
  );
}
