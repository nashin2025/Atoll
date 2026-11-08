import { gql } from '@apollo/client';

export const NETWORK_STATS_QUERY = gql`
  query NetworkStats {
    networkStats {
      blockHeight
      finalizedHeight
      blockCount
      totalSupply
      mindSupply
      peerCount
      syncing
    }
  }
`;

export const BLOCK_BY_HEIGHT_QUERY = gql`
  query BlockByHeight($height: String!) {
    blockByHeight(height: $height) {
      hash
      height
      timestamp
      shardId
      version
      difficulty
      proposer {
        address
      }
      stateRoot
      transactionRoot
      transactionCount
    }
  }
`;

export const BLOCKS_LIST_QUERY = gql`
  query BlocksList($first: String, $offset: String) {
    blocks(first: $first, offset: $offset) {
      hash
      height
      timestamp
      shardId
      proposer {
        address
      }
      transactionCount
    }
  }
`;

export const BLOCK_DETAIL_QUERY = gql`
  query BlockDetail($hash: String!) {
    block(hash: $hash) {
      hash
      height
      timestamp
      shardId
      version
      difficulty
      proposer {
        address
      }
      stateRoot
      transactionRoot
      transactionCount
      transactions {
        hash
        version
        from {
          address
        }
        to {
          address
        }
      value
      data
      blockHeight
      }
    }
  }
`;

export const TRANSACTION_BY_HASH_QUERY = gql`
  query TransactionByHash($hash: String!) {
    transaction(hash: $hash) {
      hash
      version
      from {
        address
      }
      to {
        address
      }
      value
      data
      blockHeight
    }
  }
`;

export const TRANSACTIONS_LIST_QUERY = gql`
  query TransactionsList($first: String, $offset: String) {
    transactions(first: $first, offset: $offset) {
      hash
      version
      from {
        address
      }
      to {
        address
      }
      value
        data
        blockHeight
    }
  }
`;

export const ADDRESS_QUERY = gql`
  query Address($address: String!) {
    address(address: $address) {
      address
      balance
      nonce
      codeHash
      storageRoot
      transactionCount
    }
  }
`;

export const VALIDATORS_LIST_QUERY = gql`
  query ValidatorsList($first: String) {
    validators(first: $first) {
      address {
        address
      }
      blocksProposed
      lastBlockHeight
    }
  }
`;
