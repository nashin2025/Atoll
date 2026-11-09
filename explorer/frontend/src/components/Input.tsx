import { InputHTMLAttributes } from 'react';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export function Input({ label, error, className, ...props }: InputProps) {
  return (
    <div className="space-y-1">
      {label && (
        <label className="block text-xs font-semibold text-slate-400">{label}</label>
      )}
      <input
        className={`block w-full rounded-md border px-3 py-2 text-sm text-slate-100 placeholder-slate-500 transition-colors focus:outline-none focus:ring-1 ${
          error
            ? 'border-red-500 focus:ring-red-500 focus:border-red-500'
            : 'border-slate-700 focus:ring-atoll-500 focus:border-atoll-500'
        } ${className || ''}`}
        {...props}
      />
      {error && <span className="text-xs text-red-400">{error}</span>}
    </div>
  );
}
