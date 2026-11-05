export interface Block {
  hash: string;
  height: string;
  timestamp: string;
  shard_id: string;
  version: string;
  difficulty: string;
  proposer: Address;
  state_root: string;
  transaction_root: string;
  transaction_count: number;
  transactions?: Transaction[];
}

export interface Address {
  address: string;
}

export interface Transaction {
  hash: string;
  version: string;
  from: Address;
  to?: Address;
  value: string;
  data?: string;
  block_height: string;
}

export interface AddressInfo {
  address: string;
  balance: string;
  nonce: string;
  code_hash?: string;
  storage_root?: string;
  transaction_count: number;
}

export interface Validator {
  address: Address;
  blocks_proposed: number;
  last_block_height: string;
}

export interface NetworkStats {
  block_height: string;
  finalized_height: string;
  block_count: number;
  total_supply: string;
  mind_supply: string;
  peer_count: number;
  syncing: boolean;
}

export interface PaginationParams {
  first?: string;
  offset?: string;
}
