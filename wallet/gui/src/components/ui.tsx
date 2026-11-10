import { type ComponentPropsWithoutRef, type ReactNode } from "react";

/** Primary / secondary / danger button with press feedback. */
export function Button({
  children,
  className = "",
  disabled = false,
  onClick,
  variant = "primary",
  type = "button",
  title,
  size = "md",
}: {
  children: ReactNode;
  className?: string;
  disabled?: boolean;
  onClick?: () => void;
  variant?: "primary" | "secondary" | "danger" | "ghost";
  type?: "button" | "submit" | "reset";
  title?: string;
  size?: "sm" | "md" | "lg";
}) {
  const base =
    "inline-flex items-center justify-center rounded-md font-medium transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-slate-900 disabled:opacity-50 disabled:pointer-events-none";
  const variants: Record<string, string> = {
    primary:
      "bg-indigo-600 text-white hover:bg-indigo-500 active:bg-indigo-700 focus:ring-indigo-500 shadow-sm shadow-indigo-500/20",
    secondary:
      "bg-slate-700 text-white hover:bg-slate-600 active:bg-slate-800 focus:ring-slate-500",
    danger: "bg-rose-600 text-white hover:bg-rose-500 active:bg-rose-700 focus:ring-rose-500",
    ghost: "bg-transparent text-slate-300 hover:bg-slate-800 hover:text-white focus:ring-slate-500",
  };
  const sizes: Record<string, string> = {
    sm: "h-8 px-2 text-xs",
    md: "h-10 px-4 text-sm",
    lg: "h-12 px-6 text-base",
  };
  return (
    <button
      type={type}
      className={`${base} ${variants[variant]} ${sizes[size]} ${className}`}
      disabled={disabled}
      onClick={onClick}
      title={title}
    >
      {children}
    </button>
  );
}

/** Styled text input with an optional label and error message. */
export interface InputProps extends Omit<ComponentPropsWithoutRef<"input">, "size"> {
  label?: string;
  error?: string;
}

export function Input({ label, error, id, className = "", ...props }: InputProps) {
  const inputId = id || (label ? `input-${label.toLowerCase().replace(/\s+/g, "-")}` : undefined);
  return (
    <div className="mb-4">
      {label && (
        <label
          htmlFor={inputId}
          className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1.5"
        >
          {label}
        </label>
      )}
      <input
        id={inputId}
        className={`block w-full rounded-md border border-slate-600 bg-slate-900 text-slate-100 placeholder-slate-500 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 focus:outline-none sm:text-sm transition-colors ${error ? "border-rose-500" : ""} ${className}`}
        {...props}
      />
      {error && <p className="mt-1.5 text-xs text-rose-400">{error}</p>}
    </div>
  );
}

/** Centered loading indicator. */
export function Loading({ className = "" }: { className?: string }) {
  return (
    <div className={`flex flex-col items-center justify-center py-20 ${className}`}>
      <div className="w-8 h-8 border-4 border-slate-700 border-t-indigo-500 rounded-full animate-spin" />
      <p className="mt-4 text-slate-400 text-sm">Loading…</p>
    </div>
  );
}

/** Inline skeleton for content loading states. */
export function Skeleton({
  className = "h-4",
  width = "full",
}: {
  className?: string;
  width?: string;
}) {
  const w = width === "full" ? "w-full" : `w-[${width}]`;
  return <div className={`animate-pulse rounded-md bg-slate-700/50 ${className} ${w}`} />;
}

/** Rendered command error. */
export function ErrorDisplay({ message }: { message: string }) {
  return (
    <div className="rounded-md border border-rose-500/50 bg-rose-900/20 p-4 text-rose-400 text-sm">
      {message}
    </div>
  );
}

/** Success message banner. */
export function SuccessDisplay({ message }: { message: string }) {
  return (
    <div className="rounded-md border border-emerald-500/50 bg-emerald-900/20 p-4 text-emerald-400 text-sm">
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
    shorten && address.length > 16 ? `${address.slice(0, 6)}…${address.slice(-4)}` : address;
  return (
    <div className="flex items-center gap-2">
      <code className="text-slate-300 text-sm font-mono truncate">{display}</code>
      {onCopy && (
        <Button
          variant="ghost"
          className="h-8 w-8 px-0 text-xs hover:bg-slate-800"
          onClick={onCopy}
          title="Copy address"
        >
          Copy
        </Button>
      )}
    </div>
  );
}

/** Small status badge. */
export function Badge({
  children,
  variant = "default",
  className = "",
}: {
  children: ReactNode;
  variant?: "default" | "success" | "warning" | "error" | "muted";
  className?: string;
}) {
  const variants: Record<string, string> = {
    default: "bg-slate-700 text-slate-300",
    success: "bg-emerald-500/10 text-emerald-400 border border-emerald-500/20",
    warning: "bg-amber-500/10 text-amber-400 border border-amber-500/20",
    error: "bg-rose-500/10 text-rose-400 border border-rose-500/20",
    muted: "bg-slate-800 text-slate-500",
  };
  return (
    <span
      className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-semibold border ${variants[variant]} ${className}`}
    >
      {children}
    </span>
  );
}

/** A nav link that highlights when active. */
export function NavLink({
  children,
  active,
  onClick,
  icon,
  title,
}: {
  children: ReactNode;
  active: boolean;
  onClick: () => void;
  icon?: ReactNode;
  title?: string;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      className={`flex items-center gap-2.5 px-3 py-2.5 rounded-lg text-sm font-medium transition-all ${
        active
          ? "bg-indigo-600/20 text-indigo-300 border border-indigo-500/50 shadow-sm shadow-indigo-500/10"
          : "text-slate-400 hover:text-white hover:bg-slate-800/50"
      }`}
    >
      {icon && <span className="flex items-center">{icon}</span>}
      {children}
    </button>
  );
}

/** Stateless card wrapper with consistent styling. */
export function Card({
  children,
  className = "",
  padding = "p-6",
}: {
  children: ReactNode;
  className?: string;
  padding?: "p-4" | "p-6" | "p-8";
}) {
  return (
    <div
      className={`bg-slate-900/50 border border-slate-700/50 rounded-xl ${padding} ${className}`}
    >
      {children}
    </div>
  );
}

/** Card header with title and optional description. */
export function CardHeader({ title, description }: { title: string; description?: string }) {
  return (
    <div className="mb-4">
      <h2 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">
        {title}
      </h2>
      {description && <p className="text-xs text-slate-500">{description}</p>}
    </div>
  );
}

/** A metric value with label and optional change indicator. */
export function StatCard({
  title,
  value,
  variant = "default",
  change,
}: {
  title: string;
  value: ReactNode;
  variant?: "default" | "success" | "warning" | "error" | "muted";
  change?: ReactNode;
}) {
  const variants: Record<string, string> = {
    default: "text-slate-300",
    success: "text-emerald-400",
    warning: "text-amber-400",
    error: "text-rose-400",
    muted: "text-slate-500",
  };
  return (
    <div className="bg-slate-800/50 border border-slate-700/50 rounded-xl p-4">
      <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{title}</p>
      <div className="flex items-baseline gap-1">
        <span className={`text-lg font-mono font-bold ${variants[variant]}`}>{value}</span>
        {change && <span className="text-xs text-slate-500">{change}</span>}
      </div>
    </div>
  );
}
