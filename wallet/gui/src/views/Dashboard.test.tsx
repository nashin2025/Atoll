import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { Dashboard } from '@/views/Dashboard';

vi.mock('@/tauri', () => ({
  walletBalance: vi.fn(),
  formatInvokeError: vi.fn((error: unknown) => String(error)),
}));

import { walletBalance } from '@/tauri';

beforeEach(() => {
  vi.clearAllMocks();
});

describe('Dashboard', () => {
  it('renders address and balance', async () => {
    vi.mocked(walletBalance).mockResolvedValue({ address: '0xabc123', balance: 1234 });
    render(<Dashboard address="0xabc123" passphrase="p" onLock={vi.fn()} onSend={vi.fn()} onHistory={vi.fn()} />);
    expect(screen.getByRole('heading', { name: /dashboard/i })).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByText(/1234/)).toBeInTheDocument();
    });
    expect(screen.getByText(/^ATOLL$/)).toBeInTheDocument();
  });

  it('shows offline-mode hint when balance is zero', async () => {
    vi.mocked(walletBalance).mockResolvedValue({ address: '0xabc123', balance: 0 });
    render(<Dashboard address="0xabc123" passphrase="p" onLock={vi.fn()} onSend={vi.fn()} onHistory={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByText(/offline chainstate/i)).toBeInTheDocument();
    });
  });

  it('refreshes balance on button click', async () => {
    vi.mocked(walletBalance)
      .mockResolvedValueOnce({ address: '0xabc123', balance: 5 })
      .mockResolvedValueOnce({ address: '0xabc123', balance: 42 });
    render(<Dashboard address="0xabc123" passphrase="p" onLock={vi.fn()} onSend={vi.fn()} onHistory={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByText(/5/)).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole('button', { name: /refresh balance/i }));
    await waitFor(() => {
      expect(screen.getByText(/42/)).toBeInTheDocument();
    });
    expect(walletBalance).toHaveBeenCalledTimes(2);
  });
});
