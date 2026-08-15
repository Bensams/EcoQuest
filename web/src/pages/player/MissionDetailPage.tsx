import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { ArrowLeft, CalendarDays, MapPin } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Button, ButtonLink } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { EmptyState, Notice } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { post } from '../../lib/api';
import {
  activityLabel,
  formatDateRange,
  participationStatusTone,
  statusLabel,
  statusToneOf,
} from '../../lib/format';
import type { Activity, EcoEvent } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';
import { useNotice } from '../../lib/useNotice';

export function MissionDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { data: event, loading, error, reload } = useFetch<EcoEvent>(`/api/events/${id}`);
  const [joining, setJoining] = useState(false);
  const { notice, succeed, fail } = useNotice();
  // The event payload carries no per-caller state, so the caller's own
  // participation is read from their activity list.
  const { data: activities, reload: reloadActivities } = useFetch<Activity[]>('/api/me/activities');
  const participation = activities?.find(
    (activity) => activity.event.id === id && activity.participation.status !== 'CANCELLED',
  )?.participation;

  const join = async () => {
    if (!event) return;
    setJoining(true);
    try {
      await post(`/api/events/${event.id}/join`, {});
      succeed(
        event.status === 'ACTIVE'
          ? 'Joined. This mission is under way — scan the organizer’s QR code to check in.'
          : 'Joined. You will be able to check in once the organizer starts the mission.',
      );
      await Promise.all([reload(), reloadActivities()]);
    } catch (err) {
      fail(err, 'Could not join this mission.');
      // A 409 means someone already joined in another tab; resync so the
      // button reflects reality instead of inviting a second attempt.
      await reloadActivities();
    } finally {
      setJoining(false);
    }
  };

  if (loading) return <p className="text-sm text-forest-muted">Loading mission…</p>;
  if (error || !event) return <EmptyState title="Mission not found" detail={error ?? undefined} />;

  // Mirrors the API: registration closes when the mission ends, not when it
  // starts, so a volunteer can still sign up on site and check in.
  const isOpenToJoin =
    (event.status === 'PUBLISHED' || event.status === 'ACTIVE') &&
    new Date(event.ends_at).getTime() > Date.now();

  return (
    <div>
      <PageHeader
        eyebrow="Missions"
        title={event.name}
        action={
          <ButtonLink to="/app/missions" variant="ghost" size="sm">
            <ArrowLeft className="size-4" aria-hidden="true" /> All missions
          </ButtonLink>
        }
      />
      <div className="grid gap-4 md:grid-cols-3">
        <Card className="md:col-span-2">
          <div className="mb-4 flex items-center gap-2">
            <Badge tone={statusToneOf(event.status)}>{activityLabel(event.activity_type)}</Badge>
            <Badge tone="muted">{event.status}</Badge>
          </div>
          <div className="mb-4 space-y-2 text-sm text-forest-muted">
            <p className="flex items-center gap-2">
              <MapPin className="size-4" aria-hidden="true" /> {event.location}
            </p>
            <p className="flex items-center gap-2">
              <CalendarDays className="size-4" aria-hidden="true" /> {formatDateRange(event.starts_at, event.ends_at)}
            </p>
          </div>
          <p className="whitespace-pre-wrap text-sm text-forest">{event.description || 'No description provided.'}</p>
        </Card>
        <Card className="self-start">
          <h2 className="mb-3 text-sm font-semibold text-forest">Participation</h2>
          <p className="mb-1 text-sm text-forest-muted">
            {event.registered_count} of {event.capacity} joined
          </p>
          <div className="mb-4 h-2 overflow-hidden rounded-full bg-sage-soft">
            <div
              className="h-full rounded-full bg-leaf"
              style={{ width: `${Math.min(100, (event.registered_count / Math.max(1, event.capacity)) * 100)}%` }}
            />
          </div>
          <p className="mb-4 text-sm text-forest-muted">
            Earns <span className="font-semibold text-leaf">{event.eco_points} Eco Points</span> once verified.
          </p>
          {participation ? (
            <div className="grid gap-2">
              <Button disabled className="w-full">
                Already joined
              </Button>
              <p className="flex items-center justify-center gap-2 text-sm text-forest-muted">
                Your status:
                <Badge tone={participationStatusTone(participation.status)}>
                  {statusLabel(participation.status)}
                </Badge>
              </p>
            </div>
          ) : !isOpenToJoin ? (
            <p className="text-center text-sm text-forest-muted">This mission is not open for sign-ups.</p>
          ) : event.registered_count >= event.capacity ? (
            <Button disabled className="w-full">
              Mission full
            </Button>
          ) : (
            <Button onClick={() => void join()} disabled={joining} className="w-full">
              {joining ? 'Joining…' : 'Join mission'}
            </Button>
          )}
          {notice && <div className="mt-3"><Notice tone={notice.tone} message={notice.message} /></div>}
          {event.impacts.length > 0 && (
            <div className="mt-5 border-t border-sage pt-4">
              <h3 className="mb-2 text-sm font-semibold text-forest">Expected impact</h3>
              <ul className="space-y-1 text-sm text-forest-muted">
                {event.impacts.map((impact) => (
                  <li key={`${impact.metric}-${impact.unit}`}>
                    {impact.expected_value} {impact.unit} {impact.metric.replace(/_/g, ' ')}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </Card>
      </div>
    </div>
  );
}