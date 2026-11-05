import { type ComponentPropsWithoutRef, type ReactNode } from 'react';

/** Primary / secondary / danger button with press feedback. */
export function Button({
  children,
  className = '',
  disabled = false,
  onClick,
  variant = 'primary',
  type = 'button',
  title,
}: {
  children: ReactNode;
  className?: string;
  disabled?: boolean;
  onClick?: () => void;
  variant?: 'primary' | 'secondary' | 'danger';
  type?: 'button' | 'submit' | 'reset';
  title?: string;
}) {
  const base =
    'inline-flex items-center justify-center rounded-md font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-slate-900 disabled:opacity-50 disabled:pointer-events-none';
  const variants: Record<string, string> = {
    primary:
      'bg-indigo-600 text-white hover:bg-indigo-500 active:bg-indigo-700 focus:ring-indigo-500',
    secondary:
      'bg-slate-700 text-white hover:bg-slate-600 active:bg-slate-800 focus:ring-slate-500',
    danger:
      'bg-rose-600 text-white hover:bg-rose-500 active:bg-rose-700 focus:ring-rose-500',
  };
  return (
    <button
      type={type}
      className={`${base} ${variants[variant]} ${className}`}
      disabled={disabled}
      onClick={onClick}
      title={title}
    >
      {children}
    </button>
  );
}

/** Styled text input with an optional label and error message. */
export interface InputProps extends Omit<ComponentPropsWithoutRef<'input'>, 'size'> {
  label?: string;
  error?: string;
}

export function Input({
  label,
  error,
  id,
  className = '',
  ...props
}: InputProps) {
  const inputId = id || (label ? `input-${label.toLowerCase().replace(/\s+/g, '-')}` : undefined);
  return (
    <div className="mb-4">
      {label && (
        <label htmlFor={inputId} className="block text-sm font-medium text-slate-300 mb-1">
          {label}
        </label>
      )}
      <input
        id={inputId}
        className={`block w-full rounded-md border border-slate-600 bg-slate-900 text-slate-100 placeholder-slate-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 focus:outline-none sm:text-sm ${
          error ? 'border-rose-500' : ''
        } ${className}`}
        {...props}
      />
      {error && <p className="mt-1 text-xs text-rose-400">{error}</p>}
    </div>
  );
}

/** Centered loading indicator. */
export function Loading({ className = '' }: { className?: string }) {
  return (
    <div className={`flex flex-col items-center justify-center py-20 ${className}`}>
      <div className="w-8 h-8 border-4 border-slate-700 border-t-indigo-500 rounded-full animate-spin" />
      <p className="mt-4 text-slate-400 text-sm">Loading...</p>
    </div>
  );
}

/** Rendered command error. */
export function ErrorDisplay({ message }: { message: string }) {
  return (
    <div className="rounded-md border border-rose-500/50 bg-rose-900/30 p-4 text-rose-400 text-sm">
      {message}
    </div>
  );
}

/** Address display with an inline copy button. */
export function AddressDisplay({
  address,
  shorten = true,
  onCopy,
}: {
  address: string;
  shorten?: boolean;
  onCopy?: () => void;
}) {
  const display =
    shorten && address.length > 16
      ? `${address.slice(0, 6)}…${address.slice(-4)}`
      : address;
  return (
    <div className="flex items-center gap-2">
      <code className="text-slate-300 text-sm font-mono truncate">{display}</code>
      <Button
        variant="secondary"
        className="h-8 w-8 px-0 text-xs"
        onClick={onCopy}
        title="Copy address"
      >
        Copy
      </Button>
    </div>
  );
}

/** A nav link that highlights when active. */
export function NavLink({
  children,
  active,
  onClick,
}: {
  children: ReactNode;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`flex items-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
        active
          ? 'bg-indigo-600/20 text-indigo-300 border border-indigo-500/50'
          : 'text-slate-400 hover:text-white hover:bg-slate-800/50'
      }`}
    >
      {children}
    </button>
  );
}
