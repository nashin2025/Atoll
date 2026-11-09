export function AddressDisplay({ address, truncate = true }: { address: string; truncate?: boolean }) {
  const display = truncate && address.length > 16
    ? `${address.slice(0, 6)}...${address.slice(-4)}`
    : address;

  return (
    <span className="text-xs font-mono text-slate-300 truncate max-w-[200px] block" title={address}>
      {display}
    </span>
  );
}
