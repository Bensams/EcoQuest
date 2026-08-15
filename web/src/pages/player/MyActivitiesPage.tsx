import { useState } from 'react';
import { Link } from 'react-router-dom';
import { CalendarDays, MapPin, Plus } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Card } from '../../components/ui/Card';
import { EmptyState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { Tabs } from '../../components/ui/Tabs';
import { useAuth } from '../../lib/auth';
import { activityLabel, formatDateRange, participationStatusTone, statusToneOf } from '../../lib/format';
import type { Activity } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

type TabKey = 'upcoming' | 'pending' | 'verified' | 'completed' | 'rejected';

const TABS: Array<{ key: TabKey; label: string }> = [
  { key: 'upcoming', label: 'Upcoming' },
  { key: 'pending', label: 'Pending Verification' },
  { key: 'verified', label: 'Verified' },
  { key: 'completed', label: 'Completed' },
  { key: 'rejected', label: 'Rejected' },
];

function emptyFor(tab: TabKey) {
  const messages: Record<TabKey, string> = {
    upcoming: 'No upcoming missions.',
    pending: 'No participations awaiting verification.',
    verified: 'No verified participations yet.',
    completed: 'No completed missions.',
    rejected: 'No rejected participations.',
  };
  return messages[tab];
}

const upcomingFilter = (a: Activity) => a.participation.status === 'REGISTERED' || a.participation.status === 'PENDING_VERIFICATION';
const activeFilter = (a: Activity) => a.participation.status === 'PENDING_VERIFICATION';
const verifiedFilter = (a: Activity) => a.participation.status === 'VERIFIED';
const rejectedFilter = (a: Activity) => a.participation.status === 'REJECTED';

const FILTERS: Record<TabKey, (a: Activity) => boolean> = {
  upcoming: upcomingFilter,
  pending: activeFilter,
  verified: verifiedFilter,
  completed: (a: Activity) => a.participation.status === 'VERIFIED' || a.event.status === 'COMPLETED',
  rejected: rejectedFilter,
};

export function MyActivitiesPage() {
  const { user } = useAuth();
  const { data: activities, loading, error } = useFetch<Activity[]>(
    user ? '/api/me/activities' : null,
    user?.id,
  );
  const [tab, setTab] = useState<TabKey>('upcoming');

  if (!user) return <EmptyState title="Sign in to track your activities" detail="Your past and upcoming missions appear here." />;

  const shown = (activities ?? []).filter(FILTERS[tab]);

  return (
    <div>
      <PageHeader eyebrow="My Activities" title="Your missions" />
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}

      <Tabs items={TABS} active={tab} onChange={setTab} className="mb-4" />

      {loading && <p className="text-sm text-forest-muted">Loading activities…</p>}
      {!loading && error && <p className="text-sm text-red-600">{error}</p>}
      {!loading && !error && shown.length === 0 && (
        <EmptyState title={emptyFor(tab)} />
      )}

      {!loading && !error && shown.length > 0 && (
        <ul className="space-y-3">
          {shown.map((act) => (
            <ActivityCard key={act.participation.id} activity={act} />
          ))}
        </ul>
      )}
    </div>
  );
}

function ActivityCard({ activity }: { activity: Activity }) {
  const { participation, event } = activity;
  return (
    <Card className="flex flex-wrap items-center justify-between gap-4">
      <div className="min-w-0 flex-1">
        <div className="mb-1 flex flex-wrap items-center gap-2">
          <Link to={`/app/missions/${event.id}`} className="font-semibold text-forest hover:text-leaf">
            {event.name}
          </Link>
          <Badge tone={statusToneOf(event.status)}>{event.status}</Badge>
          <Badge tone={participationStatusTone(participation.status)}>{participation.status}</Badge>
        </div>
        <p className="flex flex-wrap items-center gap-4 text-xs text-forest-muted">
          <span className="inline-flex items-center gap-1.5">
            <CalendarDays className="size-3.5" aria-hidden="true" />
            {formatDateRange(event.starts_at, event.ends_at)}
          </span>
          <span className="inline-flex items-center gap-1.5">
            <MapPin className="size-3.5" aria-hidden="true" />
            {event.location}
          </span>
          <span className="inline-flex items-center gap-1.5">
            <Plus className="size-3.5" aria-hidden="true" />
            {activityLabel(event.activity_type)}
          </span>
        </p>
      </div>
      <p className="text-sm font-medium text-forest">{event.eco_points} pts</p>
    </Card>
  );
}
