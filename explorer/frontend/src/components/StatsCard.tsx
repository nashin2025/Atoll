export function StatsCard({
  title,
  value,
  subtitle,
}: {
  title: string;
  value: string | number;
  subtitle?: string;
}) {
  return (
    <div className="card">
      <div className="text-slate-400 text-xs font-semibold uppercase tracking-wider mb-1">
        {title}
      </div>
      <div className="text-2xl font-semibold text-slate-100">{value}</div>
      {subtitle && (
        <div className="text-xs text-slate-500 mt-1">{subtitle}</div>
      )}
    </div>
  );
}
