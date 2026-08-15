import { useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { ArrowLeft, Fingerprint, QrCode, RefreshCw } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Button } from '../../components/ui/Button';
import { DataTable } from '../../components/ui/Table';
import { Card } from '../../components/ui/Card';
import { EmptyState, Notice } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { post } from '../../lib/api';
import { activityLabel, formatDateRange, participationStatusTone, statusToneOf } from '../../lib/format';
import type { EcoEvent, Participation, QrResponse, VerificationBatchResponse } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';
import { useNotice } from '../../lib/useNotice';

export function OrgEventDetailPage() {
  const { organizationId, eventId } = useParams<{ organizationId: string; eventId: string }>();
  const { data: event, error, reload } = useFetch<EcoEvent>(eventId ? `/api/events/${eventId}` : null, eventId);
  const { data: participants, reload: reloadParticipants } = useFetch<Participation[]>(
    eventId ? `/api/events/${eventId}/participants` : null,
    eventId,
  );
  const [qr, setQr] = useState<QrResponse | null>(null);
  const { notice, clear, succeed, fail } = useNotice();
  const [busy, setBusy] = useState<string | null>(null);

  if (!eventId || !organizationId) return <EmptyState title="Missing event" />;
  if (error || !event) return <EmptyState title="Event not found" detail={error ?? undefined} />;

  const verify = async (participationId: string, action: 'verify' | 'reject') => {
    clear();
    setBusy(participationId);
    try {
      const body = await post<VerificationBatchResponse>(`/api/events/${eventId}/participants/${action}`, {
        participation_ids: [participationId],
      });
      const result = body.results[0];
      // A rejection is a successful action too: it carries no points, so say so
      // rather than reporting "+0 pts".
      succeed(
        action === 'reject'
          ? 'Participation rejected. No points were awarded.'
          : result
            ? `Verified — +${result.points_awarded} pts, level ${result.player_level}.`
            : 'Verified.',
      );
      await Promise.all([reload(), reloadParticipants()]);
    } catch (err) {
      fail(err, action === 'verify' ? 'Verification failed.' : 'Rejection failed.');
    } finally {
      setBusy(null);
    }
  };

  const rotate = async () => {
    clear();
    setBusy('qr');
    try {
      const next = await post<QrResponse>(`/api/events/${eventId}/qr`, {});
      setQr(next);
      succeed(`Check-in code ready. Valid until ${new Date(next.expires_at).toLocaleString()}.`);
    } catch (err) {
      fail(err, 'Could not generate the check-in code.');
    } finally {
      setBusy(null);
    }
  };

  return (
    <div>
      <PageHeader
        eyebrow={activityLabel(event.activity_type)}
        title={event.name}
        action={
          <Link to={`/org/${organizationId}/events`} className="inline-flex items-center gap-2 text-sm font-medium text-forest-muted hover:text-forest">
            <ArrowLeft className="size-4" aria-hidden="true" /> All events
          </Link>
        }
      />
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Badge tone={statusToneOf(event.status)}>{event.status}</Badge>
        <span className="text-xs text-forest-muted">{formatDateRange(event.starts_at, event.ends_at)}</span>
      </div>
      {notice && <div className="mb-4"><Notice tone={notice.tone} message={notice.message} /></div>}
      {event.status === 'ACTIVE' && (
        <div className="mb-6 grid gap-4 md:grid-cols-2">
          <Card>
            <h2 className="mb-1 flex items-center gap-2 text-sm font-semibold text-forest">
              <Fingerprint className="size-4" aria-hidden="true" /> Check-in code
            </h2>
            <p className="mb-3 text-sm text-forest-muted">
              Rotate the code to invalidate every previous one. Players scan it in the app when at the event.
            </p>
            <Button onClick={() => void rotate()} disabled={busy === 'qr'}>
              {qr ? <><RefreshCw className="size-4" aria-hidden="true" /> Rotate code</> : <><QrCode className="size-4" aria-hidden="true" /> Generate code</>}
            </Button>
            {qr && (
              <div className="mt-4 flex items-start gap-4">
                <div className="rounded-lg border border-sage bg-white p-2" dangerouslySetInnerHTML={{ __html: qr.svg }} />
                <div className="text-sm text-forest-muted">
                  <p className="mb-1 break-all font-mono text-xs text-forest">{qr.code}</p>
                  <p>Valid until {new Date(qr.expires_at).toLocaleString()}</p>
                </div>
              </div>
            )}
          </Card>
        </div>
      )}

      <Card>
        <h2 className="mb-3 text-sm font-semibold text-forest">
          Participants ({participants?.length ?? 0})
        </h2>
        {!participants || participants.length === 0 ? (
          <EmptyState title="No participants yet" detail="Share the published event so volunteers can join." />
        ) : (
          <DataTable
            aria-label="Event participants"
            columns={[
              { key: 'player', header: 'Player', isRowHeader: true },
              { key: 'verification', header: 'Verification' },
              { key: 'registered', header: 'Registered' },
              { key: 'checkin', header: 'Check-in' },
              { key: 'actions', header: 'Actions' },
            ]}
            rows={participants}
            renderCell={(p, key) => {
              switch (key as string) {
                case 'player':
                  return (
                    <span className="font-medium text-forest">
                      {p.username ?? p.user_id.slice(0, 8)}
                    </span>
                  );
                case 'verification':
                  return (
                    <Badge tone={participationStatusTone(p.status)}>
                      {p.status.replace(/_/g, ' ')}
                    </Badge>
                  );
                case 'registered':
                  return <span className="text-forest-muted">{new Date(p.registered_at).toLocaleString()}</span>;
                case 'checkin':
                  return <span className="text-forest-muted">{p.checked_in_at ? new Date(p.checked_in_at).toLocaleString() : '—'}</span>;
                case 'actions':
                  return p.status === 'PENDING_VERIFICATION' ? (
                    <div className="flex gap-2">
                      <Button size="sm" variant="secondary" disabled={busy === p.id} onClick={() => void verify(p.id, 'verify')}>Verify</Button>
                      <Button size="sm" variant="destructive" disabled={busy === p.id} onClick={() => void verify(p.id, 'reject')}>Reject</Button>
                    </div>
                  ) : p.status === 'VERIFIED' ? (
                    <span className="text-xs text-forest-muted">Points already awarded</span>
                  ) : p.status === 'REGISTERED' ? (
                    <span className="text-xs text-forest-muted">Awaiting check-in</span>
                  ) : null;
                default:
                  return null;
              }
            }}
          />
        )}
      </Card>
    </div>
  );
}