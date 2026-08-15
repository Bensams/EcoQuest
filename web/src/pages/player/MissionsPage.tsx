import { MapPin } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Badge } from '../../components/ui/Badge';
import { Card } from '../../components/ui/Card';
import { EmptyState } from '../../components/ui/EmptyState';
import { Field, Select, TextInput } from '../../components/ui/Field';
import { PageHeader } from '../../components/ui/PageHeader';
import { useAuth } from '../../lib/auth';
import { activityLabel, formatDateRange, statusToneOf } from '../../lib/format';
import type { EcoEvent } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

const ACTIVITY_OPTIONS = [
  { value: '', label: 'All activities' },
  { value: 'TREE_PLANTING', label: 'Tree planting' },
  { value: 'WASTE_COLLECTION', label: 'Waste collection' },
  { value: 'RECYCLING', label: 'Recycling' },
  { value: 'BEACH_CLEANUP', label: 'Beach cleanup' },
  { value: 'ENERGY_SAVING', label: 'Energy saving' },
  { value: 'EDUCATION', label: 'Education' },
  { value: 'OTHER', label: 'Other' },
];

const SORT_OPTIONS = [
  { value: 'starts_at', label: 'Starting soonest' },
  { value: 'eco_points', label: 'Most Eco Points' },
  { value: 'name', label: 'Name (A–Z)' },
];

export function MissionsPage() {
  const { user } = useAuth();
  const { data: events, loading, error } = useFetch<EcoEvent[]>('/api/events');
  const [search, setSearch] = useState('');
  const [activity, setActivity] = useState('');
  const [sort, setSort] = useState('starts_at');
  const [availableOnly, setAvailableOnly] = useState(false);

  // Discovery returns every published mission, so search and filtering happen
  // here rather than as query parameters.
  const visible = useMemo(() => {
    const needle = search.trim().toLowerCase();
    const filtered = (events ?? []).filter((event) => {
      if (activity && event.activity_type !== activity) return false;
      if (availableOnly && event.registered_count >= event.capacity) return false;
      if (!needle) return true;
      return [event.name, event.location, event.organization_name, event.description]
        .filter(Boolean)
        .some((field) => String(field).toLowerCase().includes(needle));
    });
    return filtered.sort((a, b) => {
      if (sort === 'eco_points') return b.eco_points - a.eco_points;
      if (sort === 'name') return a.name.localeCompare(b.name);
      return a.starts_at.localeCompare(b.starts_at);
    });
  }, [events, search, activity, sort, availableOnly]);

  const total = events?.length ?? 0;
  const filtering = Boolean(search.trim() || activity || availableOnly);

  return (
    <div>
      <PageHeader
        eyebrow="Missions"
        title={user ? `Welcome, ${user.username}` : 'Upcoming missions'}
      />
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}
      {loading && <p className="text-sm text-forest-muted">Loading missions…</p>}

      {!loading && !error && total > 0 && (
        <div className="mb-4 grid gap-3">
          <div className="grid gap-3 md:grid-cols-3">
            <Field label="Search">
              <TextInput
                type="search"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Name, place, or organization"
              />
            </Field>
            <Field label="Activity">
              <Select value={activity} onChange={setActivity} options={ACTIVITY_OPTIONS} />
            </Field>
            <Field label="Sort by">
              <Select value={sort} onChange={setSort} options={SORT_OPTIONS} />
            </Field>
          </div>
          <div className="flex flex-wrap items-center justify-between gap-2">
            <label className="flex items-center gap-2 text-sm text-forest-muted">
              <input
                type="checkbox"
                checked={availableOnly}
                onChange={(e) => setAvailableOnly(e.target.checked)}
                className="size-4 rounded border-sage accent-leaf"
              />
              Only missions with space left
            </label>
            <p aria-live="polite" className="text-sm text-forest-muted">
              Showing {visible.length} of {total} mission{total === 1 ? '' : 's'}
            </p>
          </div>
        </div>
      )}

      {!loading && !error && total === 0 && (
        <EmptyState title="No upcoming missions" detail="Check back soon for the next local event." />
      )}
      {!loading && !error && total > 0 && visible.length === 0 && (
        <EmptyState
          title="No missions match your filters"
          detail={filtering ? 'Try a different search or clear the filters.' : undefined}
        />
      )}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {visible.map((event) => (
          <Card key={event.id} className="flex flex-col gap-3">
            <div className="flex items-start justify-between gap-2">
              <h2 className="text-base font-semibold text-forest">{event.name}</h2>
              <div className="flex shrink-0 flex-wrap justify-end gap-1.5">
                {/* Discovery now includes missions already under way; say so,
                    since those are the only ones you can check in to. */}
                {event.status === 'ACTIVE' && <Badge tone="success">Happening now</Badge>}
                <Badge tone={statusToneOf(event.status)}>{activityLabel(event.activity_type)}</Badge>
              </div>
            </div>
            <p className="flex items-center gap-1.5 text-sm text-forest-muted">
              <MapPin className="size-4" aria-hidden="true" />
              {event.location}
            </p>
            <p className="text-sm text-forest-muted">{formatDateRange(event.starts_at, event.ends_at)}</p>
            <div className="mt-auto">
              <p className="mb-1.5 text-sm text-forest-muted">
                {event.registered_count}/{event.capacity} joined · {event.eco_points} pts
              </p>
              <div className="h-1.5 overflow-hidden rounded-full bg-sage-soft">
                <div
                  className="h-full rounded-full bg-leaf"
                  style={{ width: `${Math.min(100, (event.registered_count / Math.max(1, event.capacity)) * 100)}%` }}
                />
              </div>
            </div>
            <Link
              to={`/app/missions/${event.id}`}
              className="mt-1 inline-flex items-center justify-center rounded-lg border border-sage bg-white px-4 py-2 text-sm font-medium text-forest transition-colors hover:bg-sage-soft"
            >
              View mission
            </Link>
          </Card>
        ))}
      </div>
    </div>
  );
}