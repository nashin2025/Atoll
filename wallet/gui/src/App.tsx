import { useState, useCallback } from 'react';
import { Welcome } from '@/views/Welcome';
import { Dashboard } from '@/views/Dashboard';
import { Send } from '@/views/Send';
import { History } from '@/views/History';
import { DeriveAddress } from '@/views/DeriveAddress';
import {
  nodeConnect,
  nodeDisconnect,
  NodeConnectResponse,
} from '@/tauri';

type View = 'welcome' | 'dashboard' | 'send' | 'history' | 'derive-address';

interface AppState {
  view: View;
  addresses: string[];
  selectedAddress: string | null;
  passphrase: string;
  nodeConnected: boolean;
  nodeUrl: string;
  nodeInfo: { chainId: string; tipHeight: number; version: string } | null;
}

export default function App() {
  const [state, setState] = useState<AppState>({
    view: 'welcome',
    addresses: [],
    selectedAddress: null,
    passphrase: '',
    nodeConnected: false,
    nodeUrl: 'http://localhost:8080',
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

  const handleConnectNode = useCallback(async () => {
    try {
      const result: NodeConnectResponse = await nodeConnect({
        url: state.nodeUrl,
      });
      setState((prev) => ({
        ...prev,
        nodeConnected: true,
        nodeInfo: {
          chainId: result.chain_id,
          tipHeight: result.tip_height,
          version: result.version,
        },
      }));
    } catch (err) {
      // Connection failure is non-fatal; wallet still works offline.
      console.error('Node connection failed:', err);
    }
  }, [state.nodeUrl]);

  const handleDisconnectNode = useCallback(async () => {
    try {
      await nodeDisconnect({});
    } catch {
      // Ignore errors on disconnect
    }
    setState((prev) => ({
      ...prev,
      nodeConnected: false,
      nodeInfo: null,
    }));
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
          onConnectNode={handleConnectNode}
          onDisconnectNode={handleDisconnectNode}
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
    </div>
  );
}
