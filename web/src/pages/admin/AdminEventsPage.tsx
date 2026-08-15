import { useState } from 'react';
import { CalendarClock } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { Dialog } from '../../components/ui/Dialog';
import { DataTable } from '../../components/ui/Table';
import { EmptyState, ErrorNote } from '../../components/ui/EmptyState';
import { Field, TextArea } from '../../components/ui/Field';
import { PageHeader } from '../../components/ui/PageHeader';
import { post } from '../../lib/api';
import { activityLabel, eventStatusTone, formatDateTime } from '../../lib/format';
import type { AdminEvent, Page } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

export function AdminEventsPage() {
  const { data, error, loading, reload } = useFetch<Page<AdminEvent>>('/api/admin/events');
  const events = data?.items ?? [];
  const [notice, setNotice] = useState<string | null>(null);
  const [target, setTarget] = useState<AdminEvent | null>(null);
  const [reason, setReason] = useState('');
  const [busy, setBusy] = useState(false);

  const close = () => {
    if (busy) return;
    setTarget(null);
    setReason('');
  };

  const cancelEvent = async () => {
    if (!target) return;
    setNotice(null);
    setBusy(true);
    try {
      await post(`/api/admin/events/${target.id}/cancel`, { reason: reason.trim() });
      setNotice(`“${target.name}” was cancelled.`);
      setTarget(null);
      setReason('');
      await reload();
    } catch (err) {
      setNotice(err instanceof Error ? err.message : 'Could not cancel the event.');
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <PageHeader
        eyebrow="Administration"
        title="Events"
        detail="Moderation overview. Cancel an event that should not run."
      />
      {notice && <div className="mb-4"><ErrorNote message={notice} /></div>}
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}
      {loading && <p className="mb-4 text-sm text-forest-muted">Loading events…</p>}

      <Card>
        <h2 className="mb-4 flex items-center gap-2 text-sm font-semibold text-forest">
          <CalendarClock className="size-4" aria-hidden="true" /> All events
        </h2>
        {!loading && (!events || events.length === 0) ? (
          <EmptyState title="No events yet" detail="Published and draft events will appear here." />
        ) : events && events.length > 0 ? (
          <DataTable
            aria-label="Events"
            columns={[
              { key: 'event', header: 'Event', isRowHeader: true },
              { key: 'org', header: 'Organization' },
              { key: 'owner', header: 'Owner' },
              { key: 'activity', header: 'Activity' },
              { key: 'status', header: 'Status' },
              { key: 'participants', header: 'Participants' },
              { key: 'start', header: 'Start' },
              { key: 'actions', header: 'Actions' },
            ]}
            rows={events}
            renderCell={(ev, key) => {
              switch (key as string) {
                case 'event':
                  return (
                    <div>
                      <p className="font-medium text-forest">{ev.name}</p>
                      <p className="text-xs text-forest-muted">{ev.location}</p>
                    </div>
                  );
                case 'org':
                  return <span className="text-forest-muted">{ev.organization_name}</span>;
                case 'owner':
                  return <span className="text-forest-muted">{ev.owner_username}</span>;
                case 'activity':
                  return <span className="text-forest-muted">{activityLabel(ev.activity_type)}</span>;
                case 'status':
                  return <Badge tone={eventStatusTone(ev.status)}>{ev.status}</Badge>;
                case 'participants':
                  return <span className="text-forest-muted">{ev.registered_count}</span>;
                case 'start':
                  return <span className="text-forest-muted">{formatDateTime(ev.starts_at)}</span>;
                case 'actions':
                  return ev.status === 'CANCELLED' || ev.status === 'COMPLETED' ? (
                    <span className="text-xs text-forest-muted">
                      {ev.status === 'CANCELLED' ? ev.cancellation_reason ?? 'Cancelled' : 'Completed'}
                    </span>
                  ) : (
                    <Button size="sm" variant="destructive" onClick={() => { setTarget(ev); setReason(''); }}>
                      Cancel
                    </Button>
                  );
                default:
                  return null;
              }
            }}
          />
        ) : null}
      </Card>

      <Dialog
        open={target !== null}
        title={target ? `Cancel “${target.name}”` : 'Cancel event'}
        description="Players will no longer be able to join or check in. This cannot be undone."
        onClose={close}
      >
        <div className="grid gap-4">
          <Field label="Reason">
            <TextArea
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder="Why is this event being cancelled?"
              required
            />
          </Field>
          <div className="flex justify-end gap-2">
            <Button type="button" variant="secondary" onClick={close} disabled={busy}>Keep event</Button>
            <Button type="button" variant="destructive" onClick={() => void cancelEvent()} disabled={busy || !reason.trim()}>
              {busy ? 'Cancelling…' : 'Cancel event'}
            </Button>
          </div>
        </div>
      </Dialog>
    </div>
  );
}
