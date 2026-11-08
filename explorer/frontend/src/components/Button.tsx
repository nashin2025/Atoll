import { ButtonHTMLAttributes, ReactNode } from 'react';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary';
  children: ReactNode;
}

export function Button({
  variant = 'primary',
  className,
  disabled,
  ...props
}: ButtonProps) {
  const base =
    'inline-flex items-center justify-center rounded-md px-4 py-2 text-sm font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50';
  const variants: Record<string, string> = {
    primary:
      'bg-atoll-600 text-white hover:bg-atoll-500 focus:ring-atoll-500',
    secondary:
      'border border-slate-700 bg-slate-900 text-slate-300 hover:bg-slate-800 focus:ring-atoll-500',
  };

  return (
    <button
      className={`${base} ${variants[variant]} ${className || ''}`}
      disabled={disabled}
      {...props}
    />
  );
}
