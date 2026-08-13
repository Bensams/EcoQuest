import type { ReactNode } from 'react';
import { cn } from '../../lib/utils';

export function Card({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div data-component="card" className={cn('rounded-xl border border-sage bg-surface p-5', className)}>
      {children}
    </div>
  );
}

export function StatTile({ label, value, suffix }: { label: string; value: string; suffix?: string }) {
  return (
    <div data-component="stat-tile" className="rounded-xl border border-sage bg-surface p-5">
      <p className="text-xs font-medium uppercase tracking-wide text-forest-muted">{label}</p>
      <p className="mt-2 text-2xl font-semibold text-forest">
        {value}
        {suffix && <span className="ml-1 text-sm font-normal text-forest-muted">{suffix}</span>}
      </p>
    </div>
  );
}