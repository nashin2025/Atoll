import { useQuery } from '@apollo/client';
import { gql } from '@apollo/client';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { Block } from '../types';

const BLOCKS_LIST = gql`
  query Blocks($first: String, $offset: String) {
    blocks(first: $first, offset: $offset) {
      hash
      height
      timestamp
      proposer { address }
      transaction_count
    }
  }
`;

const BLOCK_BY_HASH = gql`
  query Block($hash: String!) {
    block(hash: $hash) {
      hash
      height
      timestamp
      shard_id
      version
      difficulty
      proposer { address }
      state_root
      transaction_root
      transaction_count
      transactions {
        hash
        from { address }
        to { address }
        value
        block_height
      }
    }
  }
`;

export function BlocksList() {
  const navigate = useNavigate();
  const [offset, setOffset] = React.useState(0);

  const { data, loading, error } = useQuery<{ blocks: Block[] }>(BLOCKS_LIST, {
    variables: { first: '20', offset: offset.toString() },
  });

  if (loading) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="flex justify-center py-20">
          <div className="h-4 w-4 rounded-full border-2 border-indigo-500 border-t-transparent animate-spin"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="card p-8 text-center">
          <p className="text-red-400">Failed to load blocks</p>
        </div>
      </div>
    );
  }

  const blocks = data?.blocks ?? [];

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="mb-6">
        <h1 className="text-3xl font-bold text-slate-100">Blocks</h1>
      </div>

      <div className="card overflow-hidden">
        <table className="w-full text-left border-collapse">
          <thead>
            <tr className="border-b border-slate-800">
              <th className="table-header">Height</th>
              <th className="table-header">Hash</th>
              <th className="table-header">Proposer</th>
              <th className="table-header">Tx Count</th>
              <th className="table-header">Timestamp</th>
            </tr>
          </thead>
          <tbody>
            {blocks.map((block) => (
              <tr
                key={block.hash}
                onClick={() => navigate(`/blocks/${block.hash}`)}
                className="hover:bg-slate-800/50 cursor-pointer transition-colors"
              >
                <td className="table-cell mono">{block.height}</td>
                <td className="table-cell mono">{block.hash}</td>
                <td className="table-cell mono">{block.proposer.address}</td>
                <td className="table-cell">{block.transaction_count}</td>
                <td className="table-cell mono">
                  {new Date(parseInt(block.timestamp)).toLocaleString()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="mt-6 flex items-center justify-between">
        <button
          onClick={() => setOffset(Math.max(0, offset - 20))}
          disabled={offset === 0}
          className="btn-secondary disabled:opacity-50 disabled:pointer-events-none"
        >
          Previous
        </button>
        <button
          onClick={() => setOffset(offset + 20)}
          className="btn-secondary"
        >
          Next
        </button>
      </div>
    </div>
  );
}

export function BlockDetail() {
  const { hash } = useParams<{ hash: string }>();
  const navigate = useNavigate();

  const { data, loading, error } = useQuery<{ block: Block | null }>(
    BLOCK_BY_HASH,
    {
      variables: { hash: hash ?? '' },
      skip: !hash,
    },
  );

  if (loading) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="flex justify-center py-20">
          <div className="h-4 w-4 rounded-full border-2 border-indigo-500 border-t-transparent animate-spin"></div>
        </div>
      </div>
    );
  }

  if (error || !data?.block) {
    return (
      <div className="max-w-7xl mx-auto px-4 py-8">
        <div className="card p-8 text-center">
          <h2 className="text-xl font-semibold text-red-400 mb-2">Block not found</h2>
          <button
            onClick={() => navigate('/blocks')}
            className="btn-primary mt-4"
          >
            Back to Blocks
          </button>
        </div>
      </div>
    );
  }

  const block = data.block;

  return (
    <div className="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
      <div className="flex items-center justify-between mb-6">
        <Link to="/blocks" className="text-sm text-indigo-400 hover:text-indigo-300">
          &larr; Back to Blocks
        </Link>
        <h1 className="text-3xl font-bold text-slate-100">Block #{block.height}</h1>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
        <div className="card">
          <div className="mb-4">
            <h2 className="text-lg font-semibold mb-3">Block Header</h2>
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-slate-400 text-sm">Hash</span>
                <span className="mono">{block.hash}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400 text-sm">Height</span>
                <span className="mono">{block.height}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400 text-sm">Timestamp</span>
                <span className="mono">
                  {new Date(parseInt(block.timestamp)).toLocaleString()}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400 text-sm">Shard ID</span>
                <span className="mono">{block.shard_id}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400 text-sm">Version</span>
                <span className="mono">{block.version}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400 text-sm">Difficulty</span>
                <span className="mono">{block.difficulty}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400 text-sm">Proposer</span>
                <span className="mono">{block.proposer.address}</span>
              </div>
            </div>
          </div>
          <div className="border-t border-slate-800 pt-4">
            <h3 className="text-sm font-semibold text-slate-400 uppercase mb-2">Roots</h3>
            <div className="space-y-1">
              <div className="flex justify-between">
                <span className="text-slate-400 text-xs">State Root</span>
                <span className="mono text-xs">{block.state_root}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-slate-400 text-xs">Transaction Root</span>
                <span className="mono text-xs">{block.transaction_root}</span>
              </div>
            </div>
          </div>
        </div>

        <div className="card">
          <h2 className="text-lg font-semibold mb-4">
            Transactions ({block.transaction_count})
          </h2>
          {block.transactions && block.transactions.length > 0 ? (
            <div className="space-y-2">
              {block.transactions.map((tx) => (
                <div
                  key={tx.hash}
                  className="flex items-center justify-between p-3 bg-slate-800/50 rounded-md border border-slate-800"
                >
                  <div>
                    <div className="mono">{tx.hash}</div>
                    <div className="text-xs text-slate-500 mt-1">
                      From: {tx.from.address}&nbsp;|&nbsp;Value: {tx.value}
                    </div>
                  </div>
                  <Link
                    to={`/transactions/${tx.hash}`}
                    className="text-xs text-indigo-400 hover:text-indigo-300"
                  >
                    View
                  </Link>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-slate-500 text-sm">No transactions in this block.</p>
          )}
        </div>
      </div>
    </div>
  );
}

import React from 'react';
