import { invoke } from '@tauri-apps/api/core';

// ---------------------------------------------------------------------------
// Request / response types mirroring the Rust command definitions
// ---------------------------------------------------------------------------

export interface UnlockRequest {
  passphrase: string;
}
export interface UnlockResponse {
  addresses: string[];
}

export interface NewRequest {
  passphrase: string;
}
export interface NewResponse {
  address: string;
}

export interface ImportRequest {
  secret_key: string;
  passphrase: string;
}
export interface ImportResponse {
  address: string;
}

export interface BalanceRequest {
  address: string;
}
export interface BalanceResponse {
  address: string;
  balance: number;
}

export interface SendRequest {
  from: string;
  to: string;
  amount: string;
  token: string;
  passphrase: string;
}
export interface SendResponse {
  signed_tx: string;
}

export interface HistoryRequest {
  address: string;
}
export interface HistoryResponse {
  address: string;
  message: string;
}

export interface DeriveAddressRequest {
  public_key: string;
}
export interface DeriveAddressResponse {
  address: string;
}

// ---------------------------------------------------------------------------
// Invoke wrappers (kebab-case command names match the Rust #[tauri::command(name)])
// ---------------------------------------------------------------------------

export async function walletUnlock(req: UnlockRequest): Promise<UnlockResponse> {
  return invoke('wallet-unlock', req as unknown as Record<string, unknown>);
}

export async function walletNew(req: NewRequest): Promise<NewResponse> {
  return invoke('wallet-new', req as unknown as Record<string, unknown>);
}

export async function walletImport(req: ImportRequest): Promise<ImportResponse> {
  return invoke('wallet-import', req as unknown as Record<string, unknown>);
}

export async function walletBalance(req: BalanceRequest): Promise<BalanceResponse> {
  return invoke('wallet-balance', req as unknown as Record<string, unknown>);
}

export async function walletSend(req: SendRequest): Promise<SendResponse> {
  return invoke('wallet-send', req as unknown as Record<string, unknown>);
}

export async function walletHistory(req: HistoryRequest): Promise<HistoryResponse> {
  return invoke('wallet-history', req as unknown as Record<string, unknown>);
}

export async function walletDeriveAddress(
  req: DeriveAddressRequest,
): Promise<DeriveAddressResponse> {
  return invoke('wallet-derive-address', req as unknown as Record<string, unknown>);
}

// ---------------------------------------------------------------------------
// Error formatting for command failures
// ---------------------------------------------------------------------------

/** Extracts a human-readable message from a Tauri invoke error. */
export function formatInvokeError(error: unknown): string {
  if (error instanceof Error) {
    const message = error.message;
    if (message && (message.startsWith('{') || message.includes('"kind"'))) {
      try {
        const parsed = JSON.parse(message) as Record<string, unknown>;
        const key = Object.keys(parsed)[0];
        const value = parsed[key];
        if (typeof value === 'string') {
          return value;
        }
        if (value && typeof value === 'object' && 'message' in value) {
          return String((value as Record<string, unknown>).message);
        }
        return JSON.stringify(parsed);
      } catch {
        // Not JSON — fall through to the raw message.
      }
    }
    return message;
  }
  return String(error);
}
