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
// Embedded node request / response types mirroring the Rust command definitions
// ---------------------------------------------------------------------------

export interface NodeStartRequest {}
export interface NodeStartResponse {
  started: boolean;
  chain_id: number;
  tip_height: number;
  version: string;
}

export interface NodeStopRequest {}
export interface NodeStopResponse {
  stopped: boolean;
}

export interface NodeStatusResponse {
  running: boolean;
  chain_id: number;
  tip_height: number;
  finalized_height: number;
  block_count: number;
  peer_count: number;
  mempool_size: number;
  auto_mine: boolean;
  last_block_tx_count: number;
  genesis_hash: string;
  syncing: boolean;
  version: string;
}

export interface NodeGetBalanceRequest {
  address: string;
}
export interface NodeGetBalanceResponse {
  address: string;
  balance: number;
}

export interface NodeSendTxRequest {
  signed_tx: string;
}
export interface NodeSendTxResponse {
  accepted: boolean;
  message: string;
}

export interface NodeMineRequest {}
export interface NodeMineResponse {
  mined: boolean;
  height: number;
  txs_included: number;
  message: string;
}

export interface NodeAutoMineRequest {
  enabled: boolean;
}
export interface NodeAutoMineResponse {
  enabled: boolean;
  message: string;
}

export interface NodeAutoMineStatusRequest {}
export interface NodeAutoMineStatusResponse {
  enabled: boolean;
  interval_ms: number;
}

export interface NodeAddPeerRequest {
  address: string;
  node_id?: string;
}
export interface NodeAddPeerResponse {
  added: boolean;
  message: string;
}

export interface HistoryEntry {
  height: number;
  hash: string;
  from: string;
  to: string | null;
  value: number;
}

export interface NodeGetHistoryRequest {
  address: string;
}
export interface NodeGetHistoryResponse {
  address: string;
  transactions: HistoryEntry[];
}

// ---------------------------------------------------------------------------
// Invoke wrappers (snake-case command names match Rust function names
// registered via tauri::generate_handler!)
// Each Rust command takes `request: <RequestType>`, so the invoke payload
// must be wrapped in a `request` key matching the parameter name.
// ---------------------------------------------------------------------------

export async function walletUnlock(req: UnlockRequest): Promise<UnlockResponse> {
  return invoke('wallet_unlock', { request: req });
}

export async function walletNew(req: NewRequest): Promise<NewResponse> {
  return invoke('wallet_new', { request: req });
}

export async function walletImport(req: ImportRequest): Promise<ImportResponse> {
  return invoke('wallet_import', { request: req });
}

export async function walletBalance(req: BalanceRequest): Promise<BalanceResponse> {
  return invoke('wallet_balance', { request: req });
}

export async function walletSend(req: SendRequest): Promise<SendResponse> {
  return invoke('wallet_send', { request: req });
}

export async function walletHistory(req: HistoryRequest): Promise<HistoryResponse> {
  return invoke('wallet_history', { request: req });
}

export async function walletDeriveAddress(
  req: DeriveAddressRequest,
): Promise<DeriveAddressResponse> {
  return invoke('wallet_derive_address', { request: req });
}

export async function nodeStart(_req: NodeStartRequest): Promise<NodeStartResponse> {
  return invoke('node_start', { request: _req });
}

export async function nodeStop(_req: NodeStopRequest): Promise<NodeStopResponse> {
  return invoke('node_stop', { request: _req });
}

export async function nodeStatus(_req: NodeStopRequest): Promise<NodeStatusResponse> {
  return invoke('node_status', { request: _req });
}

export async function nodeGetBalance(
  req: NodeGetBalanceRequest,
): Promise<NodeGetBalanceResponse> {
  return invoke('node_get_balance', { request: req });
}

export async function nodeSendTx(req: NodeSendTxRequest): Promise<NodeSendTxResponse> {
  return invoke('node_send_tx', { request: req });
}

export async function nodeMine(_req: NodeMineRequest): Promise<NodeMineResponse> {
  return invoke('node_mine', { request: _req });
}

export async function nodeAutoMine(req: NodeAutoMineRequest): Promise<NodeAutoMineResponse> {
  return invoke('node_auto_mine', { request: req });
}

export async function nodeAutoMineStatus(
  _req: NodeAutoMineStatusRequest,
): Promise<NodeAutoMineStatusResponse> {
  return invoke('node_auto_mine_status', { request: _req });
}

export async function nodeAddPeer(
  req: NodeAddPeerRequest,
): Promise<NodeAddPeerResponse> {
  return invoke('node_add_peer', { request: req });
}

export async function nodeGetHistory(
  req: NodeGetHistoryRequest,
): Promise<NodeGetHistoryResponse> {
  return invoke('node_get_history', { request: req });
}

// ---------------------------------------------------------------------------
// Staking invoke wrappers
// ---------------------------------------------------------------------------

export interface StakeRequest {
  from: string;
  amount: string;
  passphrase: string;
}
export interface StakeResponse {
  signed_tx: string;
}

export interface UnstakeRequest {
  from: string;
  amount: string;
  passphrase: string;
}
export interface UnstakeResponse {
  signed_tx: string;
}

export interface StakeInfoRequest {
  address: string;
}
export interface StakeInfoResponse {
  address: string;
  staked_amount: number;
  rewards: number;
}

export async function walletStake(req: StakeRequest): Promise<StakeResponse> {
  return invoke('wallet_stake', { request: req });
}

export async function walletUnstake(req: UnstakeRequest): Promise<UnstakeResponse> {
  return invoke('wallet_unstake', { request: req });
}

export async function walletStakeInfo(
  req: StakeInfoRequest,
): Promise<StakeInfoResponse> {
  return invoke('wallet_stake_info', { request: req });
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
