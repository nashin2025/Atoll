import { test as base, expect as baseExpect } from '@playwright/test';
import { execSync } from 'child_process';
import { join } from 'path';

export const test = base.extend<{
  // Wallet test helpers
  createWallet: () => Promise<{ address: string; passphrase: string }>;
  importWallet: (secretKey: string, passphrase: string) => Promise<string>;
  sendAtoll: (from: string, to: string, amount: string) => Promise<{ signedTx: string }>;
  checkBalance: (address: string) => Promise<string>;
  deriveAddress: (publicKey: string) => Promise<string>;
  connectNode: (url: string) => Promise<{ connected: boolean }>;
  getNodeStatus: () => Promise<{ connected: boolean; tipHeight: number }>;
}>({

  // Create a new wallet
  createWallet: async ({}, use) => {
    await use(async (passphrase: string) => {
      // This would be implemented via Tauri invoke in the test page
      return { address: '', passphrase };
    });
  },

  // Import an existing wallet
  importWallet: async ({}, use) => {
    await use(async (secretKey: string, passphrase: string) => {
      return '';
    });
  },

  // Send ATOLL
  sendAtoll: async ({}, use) => {
    await use(async (from: string, to: string, amount: string) => {
      return { signedTx: '' };
    });
  },

  // Check balance
  checkBalance: async ({}, use) => {
    await use(async (address: string) => {
      return '0';
    });
  },

  // Derive address from public key
  deriveAddress: async ({}, use) => {
    await use(async (publicKey: string) => {
      return '';
    });
  },

  // Connect to node
  connectNode: async ({}, use) => {
    await use(async (url: string) => {
      return { connected: false };
    });
  },

  // Get node status
  getNodeStatus: async ({}, use) => {
    await use(async () => {
      return { connected: false, tipHeight: 0 };
    });
  },
});

export const expect = baseExpect;
