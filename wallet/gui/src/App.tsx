import { useState, useCallback } from 'react';
import { Welcome } from '@/views/Welcome';
import { Dashboard } from '@/views/Dashboard';
import { Send } from '@/views/Send';
import { History } from '@/views/History';
import { DeriveAddress } from '@/views/DeriveAddress';

type View = 'welcome' | 'dashboard' | 'send' | 'history' | 'derive-address';

interface AppState {
  view: View;
  addresses: string[];
  selectedAddress: string | null;
  passphrase: string;
}

export default function App() {
  const [state, setState] = useState<AppState>({
    view: 'welcome',
    addresses: [],
    selectedAddress: null,
    passphrase: '',
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
        />
      )}

      {state.view === 'send' && selectedAddress && (
        <Send
          from={selectedAddress}
          passphrase={passphrase}
          onBack={handleLock}
          onSendComplete={clearPassphrase}
        />
      )}

      {state.view === 'history' && (
        <History onBack={() => setState((prev) => ({ ...prev, view: 'dashboard' }))} />
      )}

      {state.view === 'derive-address' && (
        <DeriveAddress onBack={() => setState((prev) => ({ ...prev, view: 'dashboard' }))} />
      )}
    </div>
  );
}
