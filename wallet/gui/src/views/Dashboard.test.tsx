/// <reference types="@testing-library/jest-dom" />
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { Dashboard } from "@/views/Dashboard";

vi.mock("@/tauri", () => ({
  walletBalance: vi.fn(),
  nodeGetBalance: vi.fn(),
  formatInvokeError: vi.fn((error: unknown) => String(error)),
}));

import { walletBalance, nodeGetBalance } from "@/tauri";

beforeEach(() => {
  vi.clearAllMocks();
});

const defaultProps = {
  address: "0xabc123",
  passphrase: "p",
  onLock: vi.fn(),
  onSend: vi.fn(),
  onHistory: vi.fn(),
  onDeriveAddress: vi.fn(),
  nodeConnected: false,
  nodeInfo: null,
  onStartNode: vi.fn(),
  onStopNode: vi.fn(),
  onRefreshNodeStatus: vi.fn(),
  onMine: vi.fn(),
  onAutoMine: vi.fn(),
  onAddPeer: vi.fn(),
};

describe("Dashboard", () => {
  it("renders address and balance", async () => {
    vi.mocked(walletBalance).mockResolvedValue({ address: "0xabc123", balance: 1234 });
    render(<Dashboard {...defaultProps} />);
    expect(screen.queryByRole("heading", { name: /dashboard/i })).not.toBeNull();
    await waitFor(() => {
      expect(screen.queryByText(/1234/)).not.toBeNull();
    });
    expect(screen.queryByText(/^ATOLL$/)).not.toBeNull();
  });

  it("shows offline-mode hint when balance is zero", async () => {
    vi.mocked(walletBalance).mockResolvedValue({ address: "0xabc123", balance: 0 });
    render(<Dashboard {...defaultProps} />);
    await waitFor(() => {
      expect(screen.queryByText(/start the embedded node/i)).not.toBeNull();
    });
  });

  it("refreshes balance on button click", async () => {
    vi.mocked(walletBalance)
      .mockResolvedValueOnce({ address: "0xabc123", balance: 5 })
      .mockResolvedValueOnce({ address: "0xabc123", balance: 42 });
    render(<Dashboard {...defaultProps} />);
    await waitFor(() => {
      expect(screen.queryByText(/5/)).not.toBeNull();
    });
    fireEvent.click(screen.getByRole("button", { name: /refresh balance/i }));
    await waitFor(() => {
      expect(screen.queryByText(/42/)).not.toBeNull();
    });
    expect(walletBalance).toHaveBeenCalledTimes(2);
  });

  it("shows node connection info when connected", async () => {
    vi.mocked(nodeGetBalance).mockResolvedValue({ address: "0xabc123", balance: 999 });
    render(
      <Dashboard
        {...defaultProps}
        nodeConnected={true}
        nodeInfo={{
          chainId: 777777,
          tipHeight: 42,
          finalizedHeight: 40,
          blockCount: 43,
          peerCount: 3,
          mempoolSize: 0,
          autoMine: true,
          lastBlockTxCount: 0,
          genesisHash: "0x" + "0".repeat(64),
          syncing: false,
          version: "1.0.0",
        }}
      />,
    );
    await waitFor(() => {
      expect(screen.queryByText(/999/)).not.toBeNull();
    });
    expect(screen.queryByText(/777777/i)).not.toBeNull();
  });
});
