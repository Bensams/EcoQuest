import { Button } from '../ui/Button';
import { Field, Select, TextInput } from '../ui/Field';
import type { AdminQuery } from '../../lib/adminQuery';
import { pageCount } from '../../lib/adminQuery';

export function AdminToolbar({
  query,
  onChange,
  statuses,
  extra,
  total,
}: {
  query: AdminQuery;
  onChange: (next: AdminQuery) => void;
  statuses: Array<{ value: string; label: string }>;
  extra?: React.ReactNode;
  total: number;
}) {
  const pages = pageCount(total, query.perPage);
  return (
    <div className="mb-4 grid gap-3">
      <div className="grid gap-3 md:grid-cols-4">
        <Field label="Search">
          <TextInput
            value={query.q}
            onChange={(e) => onChange({ ...query, q: e.target.value, page: 1 })}
            placeholder="Name, email, or location"
          />
        </Field>
        <Field label="Status">
          <Select
            value={query.status}
            onChange={(status) => onChange({ ...query, status, page: 1 })}
            options={[{ value: '', label: 'All statuses' }, ...statuses]}
          />
        </Field>
        <Field label="Sort">
          <Select
            value={query.sort}
            onChange={(sort) => onChange({ ...query, sort, page: 1 })}
            options={[
              { value: 'created_at', label: 'Created' },
              { value: 'name', label: 'Name' },
              { value: 'status', label: 'Status' },
              { value: 'starts_at', label: 'Start date' },
              { value: 'username', label: 'Username' },
              { value: 'points', label: 'Points' },
            ]}
          />
        </Field>
        <Field label="Order">
          <Select
            value={query.order}
            onChange={(order) => onChange({ ...query, order: order === 'asc' ? 'asc' : 'desc', page: 1 })}
            options={[
              { value: 'desc', label: 'Newest first' },
              { value: 'asc', label: 'Oldest first' },
            ]}
          />
        </Field>
      </div>
      {extra}
      <div className="flex flex-wrap items-center justify-between gap-2 text-sm text-forest-muted">
        <p>{total} result{total === 1 ? '' : 's'}</p>
        <div className="flex items-center gap-2">
          <Button size="sm" variant="secondary" disabled={query.page <= 1} onClick={() => onChange({ ...query, page: query.page - 1 })}>
            Previous
          </Button>
          <span>Page {query.page} of {pages}</span>
          <Button size="sm" variant="secondary" disabled={query.page >= pages} onClick={() => onChange({ ...query, page: query.page + 1 })}>
            Next
          </Button>
        </div>
      </div>
    </div>
  );
}
