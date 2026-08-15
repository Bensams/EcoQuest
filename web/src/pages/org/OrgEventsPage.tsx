import { useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { CalendarDays, MapPin, Plus, Users } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Button, ButtonLink } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { ErrorNote, EmptyState, Notice } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { post } from '../../lib/api';
import { activityLabel, formatDateRange, statusToneOf } from '../../lib/format';
import type { ActivityType, EcoEvent, ImpactMetric } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';
import { useNotice } from '../../lib/useNotice';

export function OrgEventsPage() {
  const { organizationId } = useParams<{ organizationId: string }>();
  const { data: events, loading, error, reload } = useFetch<EcoEvent[]>(
    organizationId ? `/api/organizations/${organizationId}/events` : null,
    organizationId,
  );
  const [creating, setCreating] = useState(false);
  const { notice, clear, succeed, fail } = useNotice();

  if (!organizationId) return <EmptyState title="Missing organization" />;

  const pastTense = { publish: 'published', activate: 'activated', cancel: 'cancelled' } as const;

  const act = async (event: EcoEvent, action: 'publish' | 'activate' | 'cancel') => {
    clear();
    try {
      await post(`/api/events/${event.id}/${action}`, {});
      succeed(`“${event.name}” was ${pastTense[action]}.`);
      await reload();
    } catch (err) {
      fail(err, 'Action failed.');
    }
  };

  return (
    <div>
      <PageHeader
        eyebrow="Organization"
        title="Events"
        action={
          <Button onClick={() => { setCreating((v) => !v); clear(); }}>
            {creating ? 'Close form' : <><Plus className="size-4" aria-hidden="true" /> New event</>}
          </Button>
        }
      />
      {notice && <div className="mb-4"><Notice tone={notice.tone} message={notice.message} /></div>}
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}

      {creating && (
        <EventForm
          organizationId={organizationId}
          onCreated={() => {
            setCreating(false);
            succeed('Event created. It stays a draft until you publish it.');
            void reload();
          }}
          onCancel={() => setCreating(false)}
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

function EventForm({
  organizationId,
  onCreated,
  onCancel,
}: {
  organizationId: string;
  onCreated: () => void;
  onCancel: () => void;
}) {
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
  // Declared per verified volunteer, never a whole-event total. Rows are keyed
  // by metric, which the API also requires to be unique within one event.
  const [impacts, setImpacts] = useState<Array<{ metric: string; expected_value: string }>>([]);
  const { data: catalog } = useFetch<ImpactMetric[]>('/api/impact/metrics');

  const set = (key: keyof typeof form) => (value: string) => setForm((f) => ({ ...f, [key]: value }));

  const unused = (catalog ?? []).filter((m) => !impacts.some((i) => i.metric === m.metric));
  const definitionOf = (metric: string) => catalog?.find((m) => m.metric === metric);

  const addImpact = () => {
    const next = unused[0];
    if (next) setImpacts((rows) => [...rows, { metric: next.metric, expected_value: '' }]);
  };
  const setImpact = (index: number, patch: Partial<{ metric: string; expected_value: string }>) =>
    setImpacts((rows) => rows.map((row, i) => (i === index ? { ...row, ...patch } : row)));
  const removeImpact = (index: number) =>
    setImpacts((rows) => rows.filter((_, i) => i !== index));

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
        // The unit is never typed: it comes from the catalogue entry, so totals
        // cannot split across spellings of the same measure.
        impacts: impacts
          .filter((row) => row.expected_value.trim() !== '')
          .map((row) => ({
            metric: row.metric,
            unit: definitionOf(row.metric)?.unit ?? '',
            expected_value: Number(row.expected_value),
          })),
      });
      onCreated();
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

      <div className="mt-6 border-t border-sage pt-5">
        <h3 className="text-sm font-medium text-forest">Expected impact</h3>
        <p className="mb-3 text-xs text-forest-muted">
          What one verified volunteer contributes — not the total for the whole event. Each
          verified participant adds this much to the impact dashboard. Optional.
        </p>

        {impacts.length > 0 && (
          <div className="mb-3 grid gap-2">
            {impacts.map((row, index) => {
              const definition = definitionOf(row.metric);
              return (
                <div key={row.metric} className="flex flex-wrap items-end gap-2">
                  <label className="grid flex-1 gap-1.5 text-xs font-medium text-forest">
                    <span className="sr-only">Metric</span>
                    <select
                      value={row.metric}
                      onChange={(e) => setImpact(index, { metric: e.target.value })}
                      className={inputClass}
                      aria-label={`Impact metric ${index + 1}`}
                    >
                      {/* The row's own metric plus the ones not already used, so
                          a metric can never be listed twice. */}
                      {[definition, ...unused].filter(Boolean).map((m) => (
                        <option key={m!.metric} value={m!.metric}>{m!.label}</option>
                      ))}
                    </select>
                  </label>
                  <label className="grid w-28 gap-1.5 text-xs font-medium text-forest">
                    <span className="sr-only">Amount per volunteer</span>
                    <TextInput
                      type="number"
                      min={0}
                      max={definition?.max_per_participant}
                      step="any"
                      value={row.expected_value}
                      onChange={(e) => setImpact(index, { expected_value: e.target.value })}
                      aria-label={`Amount per volunteer for ${definition?.label ?? row.metric}`}
                    />
                  </label>
                  <span className="pb-2 text-sm text-forest-muted">{definition?.unit}</span>
                  <Button variant="ghost" size="sm" onClick={() => removeImpact(index)} className="mb-0.5">
                    Remove
                  </Button>
                </div>
              );
            })}
          </div>
        )}

        <Button variant="secondary" size="sm" onClick={addImpact} disabled={unused.length === 0}>
          <Plus className="size-3.5" aria-hidden="true" /> Add impact metric
        </Button>
      </div>

      <div className="mt-5 flex justify-end gap-2">
        <Button variant="secondary" onClick={onCancel}>Cancel</Button>
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