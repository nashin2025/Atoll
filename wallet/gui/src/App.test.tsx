/// <reference types="@testing-library/jest-dom" />
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import App from "@/App";

vi.mock("@/tauri", () => ({
  walletUnlock: vi.fn(),
  walletNew: vi.fn(),
  walletImport: vi.fn(),
  walletBalance: vi.fn(),
  walletSend: vi.fn(),
  walletHistory: vi.fn(),
  walletDeriveAddress: vi.fn(),
  formatInvokeError: vi.fn((error: unknown) => String(error)),
}));

import { walletUnlock } from "@/tauri";

beforeEach(() => {
  vi.clearAllMocks();
});

describe("App", () => {
  it("renders the welcome view initially", () => {
    render(<App />);
    expect(screen.queryByRole("heading", { name: /atoll wallet/i })).not.toBeNull();
    expect(screen.queryByRole("button", { name: /unlock wallet/i })).not.toBeNull();
    expect(screen.queryByRole("button", { name: /create new wallet/i })).not.toBeNull();
    expect(screen.queryByRole("button", { name: /import existing key/i })).not.toBeNull();
  });

  it("navigates to the create-wallet view", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /create new wallet/i }));
    expect(screen.queryByRole("heading", { name: /create new wallet/i })).not.toBeNull();
    expect(screen.queryByLabelText(/passphrase to encrypt keystore/i)).not.toBeNull();
  });

  it("navigates to the import-wallet view", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /import existing key/i }));
    expect(screen.queryByRole("heading", { name: /import wallet/i })).not.toBeNull();
    expect(screen.queryByLabelText(/secret key \(hex\)/i)).not.toBeNull();
  });

  it("calls wallet-unlock on unlock and switches to dashboard", async () => {
    vi.mocked(walletUnlock).mockResolvedValue({ addresses: ["0xabcdef0123456789"] });
    render(<App />);
    fireEvent.change(screen.getByLabelText(/passphrase/i), {
      target: { value: "test-passphrase" },
    });
    fireEvent.click(screen.getByRole("button", { name: /unlock wallet/i }));
    await vi.waitFor(() => {
      expect(screen.getAllByRole("heading", { name: /dashboard/i }).length).toBeGreaterThanOrEqual(1);
    });
    expect(walletUnlock).toHaveBeenCalledTimes(1);
  });
});
