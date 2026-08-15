import { useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { CalendarDays, MapPin, Plus, Users } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Button, ButtonLink } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { ErrorNote, EmptyState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { post } from '../../lib/api';
import { activityLabel, formatDateRange, statusToneOf } from '../../lib/format';
import type { ActivityType, EcoEvent } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

export function OrgEventsPage() {
  const { organizationId } = useParams<{ organizationId: string }>();
  const { data: events, loading, error, reload } = useFetch<EcoEvent[]>(
    organizationId ? `/api/organizations/${organizationId}/events` : null,
    organizationId,
  );
  const [creating, setCreating] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  if (!organizationId) return <EmptyState title="Missing organization" />;

  const act = async (event: EcoEvent, action: 'publish' | 'activate' | 'cancel') => {
    setNotice(null);
    try {
      await post(`/api/events/${event.id}/${action}`, {});
      setNotice(`${action[0].toUpperCase()}${action.slice(1)} requested for “${event.name}”.`);
      await reload();
    } catch (err) {
      setNotice(err instanceof Error ? err.message : 'Action failed.');
    }
  };

  return (
    <div>
      <PageHeader
        eyebrow="Organization"
        title="Events"
        action={
          <Button onClick={() => { setCreating((v) => !v); setNotice(null); }}>
            {creating ? 'Close form' : <><Plus className="size-4" aria-hidden="true" /> New event</>}
          </Button>
        }
      />
      {notice && <div className="mb-4"><ErrorNote message={notice} /></div>}
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}

      {creating && (
        <EventForm
          organizationId={organizationId}
          onDone={() => { setCreating(false); setNotice('Event created.'); void reload(); }}
        />
      )}

      {loading && <p className="text-sm text-forest-muted">Loading events…</p>}
      {!loading && events && events.length === 0 && (
        <EmptyState
          title="No events yet"
          detail="Create your first event to start recruiting volunteers and tracking impact."
        />
      )}

      <div className="space-y-3">
        {(events ?? []).map((event) => (
          <Card key={event.id} className="flex flex-wrap items-center justify-between gap-4">
            <div>
              <div className="mb-1 flex flex-wrap items-center gap-2">
                <Link to={`/org/${organizationId}/events/${event.id}`} className="font-semibold text-forest hover:text-leaf">
                  {event.name}
                </Link>
                <Badge tone={statusToneOf(event.status)}>{event.status}</Badge>
                <Badge tone="muted">{activityLabel(event.activity_type)}</Badge>
              </div>
              <p className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-forest-muted">
                <span className="inline-flex items-center gap-1.5"><MapPin className="size-3.5" aria-hidden="true" />{event.location}</span>
                <span className="inline-flex items-center gap-1.5"><CalendarDays className="size-3.5" aria-hidden="true" />{formatDateRange(event.starts_at, event.ends_at)}</span>
                <span className="inline-flex items-center gap-1.5"><Users className="size-3.5" aria-hidden="true" />{event.registered_count}/{event.capacity} joined</span>
              </p>
            </div>
            <div className="flex items-center gap-2">
              <ButtonLink to={`/org/${organizationId}/events/${event.id}`} variant="secondary" size="sm">Manage</ButtonLink>
              {event.status === 'DRAFT' && (
                <Button size="sm" variant="secondary" onClick={() => void act(event, 'publish')}>Publish</Button>
              )}
              {event.status === 'PUBLISHED' && (
                <Button size="sm" variant="secondary" onClick={() => void act(event, 'activate')}>Activate</Button>
              )}
              {(event.status === 'PUBLISHED' || event.status === 'ACTIVE') && (
                <Button size="sm" variant="destructive" onClick={() => void act(event, 'cancel')}>Cancel</Button>
              )}
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
}

const activityTypes: ActivityType[] = [
  'TREE_PLANTING',
  'WASTE_COLLECTION',
  'RECYCLING',
  'BEACH_CLEANUP',
  'ENERGY_SAVING',
  'EDUCATION',
  'OTHER',
];

function EventForm({ organizationId, onDone }: { organizationId: string; onDone: () => void }) {
  const [form, setForm] = useState({
    name: '',
    description: '',
    activity_type: 'TREE_PLANTING' as ActivityType,
    location: '',
    starts_at: '',
    ends_at: '',
    capacity: '20',
    eco_points: '100',
  });
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const set = (key: keyof typeof form) => (value: string) => setForm((f) => ({ ...f, [key]: value }));

  const submit = async () => {
    setError(null);
    setSubmitting(true);
    try {
      await post(`/api/organizations/${organizationId}/events`, {
        ...form,
        capacity: Number(form.capacity),
        eco_points: Number(form.eco_points),
        starts_at: new Date(form.starts_at).toISOString(),
        ends_at: new Date(form.ends_at).toISOString(),
        impacts: [],
      });
      onDone();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not create the event.');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Card className="mb-6">
      <h2 className="mb-4 text-sm font-semibold text-forest">New event</h2>
      {error && <div className="mb-4"><ErrorNote message={error} /></div>}
      <div className="grid gap-4 md:grid-cols-2">
        <Field label="Name"><TextInput value={form.name} onChange={(e) => set('name')(e.target.value)} placeholder="Riverbank cleanup" /></Field>
        <Field label="Activity type">
          <select value={form.activity_type} onChange={(e) => set('activity_type')(e.target.value)} className={inputClass}>
            {activityTypes.map((t) => <option key={t} value={t}>{activityLabel(t)}</option>)}
          </select>
        </Field>
        <div className="md:col-span-2">
          <Field label="Description"><textarea value={form.description} onChange={(e) => set('description')(e.target.value)} className={`${inputClass} min-h-20 resize-y`} /></Field>
        </div>
        <div className="md:col-span-2">
          <Field label="Location"><TextInput value={form.location} onChange={(e) => set('location')(e.target.value)} placeholder="Greenfield Park" /></Field>
        </div>
        <Field label="Starts at"><TextInput type="datetime-local" value={form.starts_at} onChange={(e) => set('starts_at')(e.target.value)} /></Field>
        <Field label="Ends at"><TextInput type="datetime-local" value={form.ends_at} onChange={(e) => set('ends_at')(e.target.value)} /></Field>
        <Field label="Capacity"><TextInput type="number" min={1} value={form.capacity} onChange={(e) => set('capacity')(e.target.value)} /></Field>
        <Field label="Eco points per volunteer"><TextInput type="number" min={0} value={form.eco_points} onChange={(e) => set('eco_points')(e.target.value)} /></Field>
      </div>
      <div className="mt-5 flex justify-end gap-2">
        <Button variant="secondary" onClick={onDone}>Cancel</Button>
        <Button onClick={() => void submit()} disabled={submitting || !form.name || !form.starts_at || !form.ends_at}>
          {submitting ? 'Creating…' : 'Create event'}
        </Button>
      </div>
    </Card>
  );
}

const inputClass =
  'rounded-lg border border-sage bg-white px-3 py-2 text-sm text-forest placeholder:text-forest-muted/60 outline-none transition focus:border-leaf focus:ring-2 focus:ring-leaf/20';

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="grid gap-1.5 text-sm font-medium text-forest">
      <span>{label}</span>
      {children}
    </label>
  );
}

function TextInput(props: React.InputHTMLAttributes<HTMLInputElement>) {
  return <input className={inputClass} {...props} />;
}