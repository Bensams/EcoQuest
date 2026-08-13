import { MapPin } from 'lucide-react';
import { Link } from 'react-router-dom';
import { Badge } from '../../components/ui/Badge';
import { Card } from '../../components/ui/Card';
import { EmptyState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { useAuth } from '../../lib/auth';
import { activityLabel, formatDateRange, statusToneOf } from '../../lib/format';
import type { EcoEvent } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

export function MissionsPage() {
  const { user } = useAuth();
  const { data: events, loading, error } = useFetch<EcoEvent[]>('/api/events');

  return (
    <div>
      <PageHeader
        eyebrow="Missions"
        title={user ? `Welcome, ${user.username}` : 'Upcoming missions'}
      />
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}
      {loading && <p className="text-sm text-forest-muted">Loading missions…</p>}
      {!loading && !error && events && events.length === 0 && (
        <EmptyState title="No upcoming missions" detail="Check back soon for the next local event." />
      )}
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {(events ?? []).map((event) => (
          <Card key={event.id} className="flex flex-col gap-3">
            <div className="flex items-start justify-between gap-2">
              <h2 className="text-base font-semibold text-forest">{event.name}</h2>
              <Badge tone={statusToneOf(event.status)}>{activityLabel(event.activity_type)}</Badge>
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