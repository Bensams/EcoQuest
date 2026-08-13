import { useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { ArrowLeft, Fingerprint, QrCode, RefreshCw } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { EmptyState, ErrorNote } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { post } from '../../lib/api';
import { activityLabel, formatDateRange, participationStatusTone, statusToneOf } from '../../lib/format';
import type { EcoEvent, Participation, QrResponse, VerificationBatchResponse } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

export function OrgEventDetailPage() {
  const { organizationId, eventId } = useParams<{ organizationId: string; eventId: string }>();
  const { data: event, error, reload } = useFetch<EcoEvent>(eventId ? `/api/events/${eventId}` : null, eventId);
  const { data: participants, reload: reloadParticipants } = useFetch<Participation[]>(
    eventId ? `/api/events/${eventId}/participants` : null,
    eventId,
  );
  const [qr, setQr] = useState<QrResponse | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  if (!eventId || !organizationId) return <EmptyState title="Missing event" />;
  if (error || !event) return <EmptyState title="Event not found" detail={error ?? undefined} />;

  const verify = async (participationId: string, action: 'verify' | 'reject') => {
    setNotice(null);
    setBusy(participationId);
    try {
      const body = await post<VerificationBatchResponse>(`/api/events/${eventId}/participants/${action}`, {
        participation_ids: [participationId],
      });
      const result = body.results[0];
      setNotice(
        result
          ? `${action === 'verify' ? 'Verified' : 'Rejected'} — +${result.points_awarded} pts, level ${result.player_level}.`
          : `${action === 'verify' ? 'Verified' : 'Rejected'}.`,
      );
      await Promise.all([reload(), reloadParticipants()]);
    } catch (err) {
      setNotice(err instanceof Error ? err.message : 'Verification failed.');
    } finally {
      setBusy(null);
    }
  };

  const rotate = async () => {
    setNotice(null);
    setBusy('qr');
    try {
      setQr(await post<QrResponse>(`/api/events/${eventId}/qr`, {}));
    } catch (err) {
      setNotice(err instanceof Error ? err.message : 'Could not generate the check-in code.');
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
      {notice && <div className="mb-4"><ErrorNote message={notice} /></div>}
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
        {!participants || (participants.length === 0 && (
          <EmptyState title="No participants yet" detail="Share the published event so volunteers can join." />
        ))}
        {participants && participants.length > 0 && (
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
               <thead>
                 <tr className="border-b border-sage text-xs uppercase tracking-wide text-forest-muted">
                   <th className="py-2 pr-4 font-medium">Player</th>
                   <th className="py-2 pr-4 font-medium">Verification</th>
                   <th className="py-2 pr-4 font-medium">Registered</th>
                   <th className="py-2 pr-4 font-medium">Check-in</th>
                   <th className="py-2 font-medium">Actions</th>
                 </tr>
               </thead>
               <tbody className="divide-y divide-sage">
                 {participants.map((p) => (
                   <tr key={p.id}>
                     <td className="py-2.5 pr-4 font-medium text-forest">{p.username ?? p.user_id.slice(0, 8)}</td>
                     <td className="py-2.5 pr-4">                     <Badge tone={participationStatusTone(p.status)}>{p.status.replace(/_/g, ' ')}</Badge></td>
                     <td className="py-2.5 pr-4 text-forest-muted">{new Date(p.registered_at).toLocaleString()}</td>
                     <td className="py-2.5 pr-4 text-forest-muted">{p.checked_in_at ? new Date(p.checked_in_at).toLocaleString() : '—'}</td>
                    <td className="py-2.5">
                      {p.status === 'PENDING_VERIFICATION' ? (
                        <div className="flex gap-2">
                          <Button size="sm" variant="secondary" disabled={busy === p.id} onClick={() => void verify(p.id, 'verify')}>Verify</Button>
                          <Button size="sm" variant="destructive" disabled={busy === p.id} onClick={() => void verify(p.id, 'reject')}>Reject</Button>
                        </div>
                      ) : p.status === 'VERIFIED' ? (
                        <span className="text-xs text-forest-muted">Points already awarded</span>
                      ) : p.status === 'REGISTERED' ? (
                        <span className="text-xs text-forest-muted">Awaiting check-in</span>
                      ) : null}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
    </div>
  );
}