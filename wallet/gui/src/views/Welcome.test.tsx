import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { Welcome } from '@/views/Welcome';

vi.mock('@/tauri', () => ({
  walletUnlock: vi.fn(),
  walletNew: vi.fn(),
  walletImport: vi.fn(),
  formatInvokeError: vi.fn((error: unknown) => String(error)),
}));

import { walletUnlock, walletNew, walletImport } from '@/tauri';

beforeEach(() => {
  vi.clearAllMocks();
});

describe('Welcome', () => {
  it('renders the unlock form by default', () => {
    render(<Welcome onUnlocked={vi.fn()} />);
    expect(screen.getByRole('heading', { name: /atoll wallet/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/passphrase/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /unlock wallet/i })).toBeInTheDocument();
  });

  it('calls wallet-unlock with the passphrase and notifies parent', async () => {
    const onUnlocked = vi.fn();
    vi.mocked(walletUnlock).mockResolvedValue({ addresses: ['0xabc123'] });
    render(<Welcome onUnlocked={onUnlocked} />);
    fireEvent.change(screen.getByLabelText(/passphrase/i), {
      target: { value: 'test-passphrase' },
    });
    fireEvent.click(screen.getByRole('button', { name: /unlock wallet/i }));
    await vi.waitFor(() => {
      expect(walletUnlock).toHaveBeenCalledWith({ passphrase: 'test-passphrase' });
    });
    expect(onUnlocked).toHaveBeenCalledWith(['0xabc123'], 'test-passphrase');
  });

  it('calls wallet_new on create and notifies parent', async () => {
    const onUnlocked = vi.fn();
    vi.mocked(walletNew).mockResolvedValue({ address: '0xnewaddress' });
    render(<Welcome onUnlocked={onUnlocked} />);
    fireEvent.click(screen.getByRole('button', { name: /create new wallet/i }));
    fireEvent.change(screen.getByLabelText(/passphrase to encrypt/i), {
      target: { value: 'create-passphrase' },
    });
    fireEvent.click(screen.getByRole('button', { name: /create wallet/i }));
    await vi.waitFor(() => {
      expect(walletNew).toHaveBeenCalledWith({ passphrase: 'create-passphrase' });
    });
    expect(onUnlocked).toHaveBeenCalledWith(['0xnewaddress'], 'create-passphrase');
  });

  it('calls wallet-import on import and notifies parent', async () => {
    const onUnlocked = vi.fn();
    vi.mocked(walletImport).mockResolvedValue({ address: '0ximported' });
    render(<Welcome onUnlocked={onUnlocked} />);
    fireEvent.click(screen.getByRole('button', { name: /import existing key/i }));
    fireEvent.change(screen.getByLabelText(/secret key \(hex\)/i), {
      target: { value: '03'.repeat(32) },
    });
    fireEvent.change(screen.getByLabelText(/passphrase to encrypt/i), {
      target: { value: 'import-passphrase' },
    });
    fireEvent.click(screen.getByRole('button', { name: /import wallet/i }));
    await vi.waitFor(() => {
      expect(walletImport).toHaveBeenCalledWith({
        secret_key: '03'.repeat(32),
        passphrase: 'import-passphrase',
      });
    });
    expect(onUnlocked).toHaveBeenCalledWith(['0ximported'], 'import-passphrase');
  });

  it('displays an error when unlock fails', async () => {
    vi.mocked(walletUnlock).mockRejectedValue(new Error('wrong passphrase'));
    render(<Welcome onUnlocked={vi.fn()} />);
    fireEvent.change(screen.getByLabelText(/passphrase/i), {
      target: { value: 'test-passphrase' },
    });
    fireEvent.click(screen.getByRole('button', { name: /unlock wallet/i }));
    await vi.waitFor(() => {
      expect(screen.getByText(/wrong passphrase/i)).toBeInTheDocument();
    });
  });
});
