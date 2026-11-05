import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import App from '@/App';

vi.mock('@/tauri', () => ({
  walletUnlock: vi.fn(),
  walletNew: vi.fn(),
  walletImport: vi.fn(),
  walletBalance: vi.fn(),
  walletSend: vi.fn(),
  walletHistory: vi.fn(),
  walletDeriveAddress: vi.fn(),
  formatInvokeError: vi.fn((error: unknown) => String(error)),
}));

import { walletUnlock } from '@/tauri';

beforeEach(() => {
  vi.clearAllMocks();
});

describe('App', () => {
  it('renders the welcome view initially', () => {
    render(<App />);
    expect(screen.getByRole('heading', { name: /atoll wallet/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /unlock wallet/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /create new wallet/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /import existing key/i })).toBeInTheDocument();
  });

  it('navigates to the create-wallet view', () => {
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: /create new wallet/i }));
    expect(screen.getByRole('heading', { name: /create new wallet/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/passphrase to encrypt keystore/i)).toBeInTheDocument();
  });

  it('navigates to the import-wallet view', () => {
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: /import existing key/i }));
    expect(screen.getByRole('heading', { name: /import wallet/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/secret key \(hex\)/i)).toBeInTheDocument();
  });

  it('calls wallet-unlock on unlock and switches to dashboard', async () => {
    vi.mocked(walletUnlock).mockResolvedValue({ addresses: ['0xabcdef0123456789'] });
    render(<App />);
    fireEvent.change(screen.getByLabelText(/passphrase/i), {
      target: { value: 'test-passphrase' },
    });
    fireEvent.click(screen.getByRole('button', { name: /unlock wallet/i }));
    await vi.waitFor(() => {
      expect(screen.getByRole('heading', { name: /dashboard/i })).toBeInTheDocument();
    });
    expect(walletUnlock).toHaveBeenCalledTimes(1);
  });
});
