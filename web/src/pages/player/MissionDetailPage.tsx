import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { ArrowLeft, CalendarDays, MapPin } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Button, ButtonLink } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { EmptyState, PageHeader } from '../../components/ui/EmptyState';
import { post } from '../../lib/api';
import { activityLabel, formatDateRange, statusToneOf } from '../../lib/format';
import type { EcoEvent } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

export function MissionDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { data: event, loading, error, reload } = useFetch<EcoEvent>(`/api/events/${id}`);
  const [joining, setJoining] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  const join = async () => {
    if (!event) return;
    setJoining(true);
    setResult(null);
    try {
      await post(`/api/events/${event.id}/join`, {});
      setResult('Joined. You will be able to check in when the event is active.');
      await reload();
    } catch (err) {
      setResult(err instanceof Error ? err.message : 'Could not join this mission.');
    } finally {
      setJoining(false);
    }
  };

  if (loading) return <p className="text-sm text-forest-muted">Loading mission…</p>;
  if (error || !event) return <EmptyState title="Mission not found" detail={error ?? undefined} />;

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
            <Badge tone="gray">{event.status}</Badge>
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
          {event.status === 'PUBLISHED' ? (
            <Button onClick={() => void join()} disabled={joining} className="w-full">
              {joining ? 'Joining…' : 'Join mission'}
            </Button>
          ) : (
            <p className="text-center text-sm text-forest-muted">This mission is not open for sign-ups.</p>
          )}
          {result && <p className="mt-3 rounded-lg bg-sage-soft px-3 py-2 text-sm text-forest">{result}</p>}
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