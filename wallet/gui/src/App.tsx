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
  NodeStartResponse,
  NodeStatusResponse,
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
