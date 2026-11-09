interface ErrorDisplayProps {
  message: string;
  onRetry?: () => void;
}

export function ErrorDisplay({ message, onRetry }: ErrorDisplayProps) {
  return (
    <div className="rounded-xl border border-red-500/50 bg-red-500/10 p-4 text-sm text-red-200">
      <p className="font-medium">{message}</p>
      {onRetry && (
        <button
          onClick={onRetry}
          className="mt-3 inline-flex items-center justify-center rounded-md border border-slate-700 bg-slate-900 px-4 py-2 text-sm font-medium text-slate-300 transition-colors hover:bg-slate-800 focus:outline-none focus:ring-2 focus:ring-atoll-500 focus:ring-offset-2"
        >
          Retry
        </button>
      )}
    </div>
  );
}
