import { formatDateTime } from '../../lib/format';
import type { AdminAuditEntry } from '../../lib/types';
import { EmptyState } from '../ui/EmptyState';

export function AuditList({ entries }: { entries: AdminAuditEntry[] }) {
  if (entries.length === 0) {
    return <EmptyState title="No audit entries yet" detail="Status changes and corrections will appear here." />;
  }
  return (
    <ol className="grid gap-3">
      {entries.map((entry) => (
        <li key={entry.id} className="rounded-lg border border-sage px-3 py-2 text-sm">
          <div className="flex flex-wrap items-baseline justify-between gap-2">
            <p className="font-medium text-forest">{entry.action}</p>
            <p className="text-xs text-forest-muted">{formatDateTime(entry.created_at)}</p>
          </div>
          <p className="text-xs text-forest-muted">
            {entry.actor_username ?? 'system'}
            {typeof entry.metadata.reason === 'string' ? ` · ${entry.metadata.reason}` : ''}
          </p>
          {(entry.metadata.previous !== undefined || entry.metadata.new !== undefined) && (
            <p className="mt-1 text-xs text-forest-muted">
              {String(entry.metadata.previous ?? '—')} → {String(entry.metadata.new ?? '—')}
            </p>
          )}
        </li>
      ))}
    </ol>
  );
}
