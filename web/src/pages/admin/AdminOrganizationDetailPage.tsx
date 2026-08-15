import { useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import { AuditList } from '../../components/admin/AuditList';
import { ReasonDialog } from '../../components/admin/ReasonDialog';
import { Badge } from '../../components/ui/Badge';
import { Button } from '../../components/ui/Button';
import { Card, StatTile } from '../../components/ui/Card';
import { EmptyState, LoadingState, Notice } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { DataTable } from '../../components/ui/Table';
import { Tabs } from '../../components/ui/Tabs';
import { patch } from '../../lib/api';
import { eventStatusTone, formatDateTime, statusLabel, verificationToneOf } from '../../lib/format';
import type { AdminOrganizationDetail, VerificationStatus } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';
import { useNotice } from '../../lib/useNotice';

type TabKey = 'profile' | 'events' | 'members' | 'certificates' | 'impact' | 'audit';

export function AdminOrganizationDetailPage() {
  const { organizationId } = useParams<{ organizationId: string }>();
  const { data, error, loading, reload } = useFetch<AdminOrganizationDetail>(
    organizationId ? `/api/admin/organizations/${organizationId}` : null,
    organizationId,
  );
  const [tab, setTab] = useState<TabKey>('profile');
  const { notice, clear, succeed, fail } = useNotice();
  const [nextStatus, setNextStatus] = useState<VerificationStatus | null>(null);
  const [busy, setBusy] = useState(false);

  if (!organizationId) return <EmptyState title="Missing organization" />;
  if (loading) return <LoadingState label="Loading organization…" />;
  if (error || !data) return <EmptyState title="Organization not found" detail={error ?? undefined} />;

  const org = data.organization;
  const documents = Array.isArray(org.supporting_documents) ? org.supporting_documents : [];

  const review = async (reason: string) => {
    if (!nextStatus) return;
    setBusy(true);
    clear();
    try {
      await patch(`/api/admin/organizations/${org.id}/status`, { status: nextStatus, reason });
      succeed(`Status is now ${statusLabel(nextStatus).toLowerCase()}.`);
      setNextStatus(null);
      await reload();
    } catch (err) {
      fail(err, 'Review failed.');
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <PageHeader
        eyebrow="Organization"
        title={org.name}
        detail={org.description || org.location}
        action={
          <Link to="/admin/organizations" className="inline-flex items-center gap-2 text-sm font-medium text-forest-muted hover:text-forest">
            <ArrowLeft className="size-4" aria-hidden="true" /> All organizations
          </Link>
        }
      />
      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Badge tone={verificationToneOf(org.verification_status)}>{statusLabel(org.verification_status)}</Badge>
        <span className="text-xs text-forest-muted">{org.organization_type} · {org.location}</span>
      </div>
      {notice && <div className="mb-4"><Notice tone={notice.tone} message={notice.message} /></div>}
      <div className="mb-4 flex flex-wrap gap-2">
        {org.verification_status === 'PENDING' && (
          <>
            <Button size="sm" onClick={() => setNextStatus('APPROVED')}>Approve</Button>
            <Button size="sm" variant="destructive" onClick={() => setNextStatus('REJECTED')}>Reject</Button>
          </>
        )}
        {org.verification_status === 'APPROVED' && (
          <>
            <Button size="sm" variant="destructive" onClick={() => setNextStatus('SUSPENDED')}>Suspend</Button>
            <Button size="sm" variant="secondary" onClick={() => setNextStatus('INACTIVE')}>Deactivate</Button>
          </>
        )}
        {org.verification_status === 'SUSPENDED' && (
          <Button size="sm" onClick={() => setNextStatus('APPROVED')}>Reactivate</Button>
        )}
      </div>

      <Tabs
        className="mb-6"
        active={tab}
        onChange={setTab}
        items={[
          { key: 'profile', label: 'Profile' },
          { key: 'events', label: `Events (${data.events.length})` },
          { key: 'members', label: `Members (${data.members.length})` },
          { key: 'certificates', label: `Certificates (${data.certificates.length})` },
          { key: 'impact', label: 'Impact' },
          { key: 'audit', label: 'Audit' },
        ]}
      />

      {tab === 'profile' && (
        <Card>
          <dl className="grid gap-3 text-sm md:grid-cols-2">
            <div><dt className="text-forest-muted">Owner</dt><dd>{org.owner_username ?? '—'}</dd></div>
            <div><dt className="text-forest-muted">Last review</dt><dd>{org.reviewed_at ? formatDateTime(org.reviewed_at) : '—'}</dd></div>
            <div className="md:col-span-2"><dt className="text-forest-muted">Review reason</dt><dd>{org.review_reason ?? '—'}</dd></div>
            <div className="md:col-span-2">
              <dt className="text-forest-muted">Supporting documents</dt>
              <dd>{documents.length === 0 ? 'None uploaded' : documents.map((doc) => String(doc)).join(', ')}</dd>
            </div>
          </dl>
        </Card>
      )}

      {tab === 'events' && (
        data.events.length === 0 ? <EmptyState title="No events" /> : (
          <DataTable
            aria-label="Organization events"
            columns={[
              { key: 'name', header: 'Event', isRowHeader: true },
              { key: 'status', header: 'Status' },
              { key: 'start', header: 'Start' },
            ]}
            rows={data.events}
            renderCell={(event, key) => {
              if (key === 'name') return <Link to={`/admin/events/${event.id}`} className="font-medium text-forest hover:text-leaf">{event.name}</Link>;
              if (key === 'status') return <Badge tone={eventStatusTone(event.status)}>{statusLabel(event.status)}</Badge>;
              return <span className="text-forest-muted">{formatDateTime(event.starts_at)}</span>;
            }}
          />
        )
      )}

      {tab === 'members' && (
        data.members.length === 0 ? <EmptyState title="No members" detail="Ownership is the only membership model." /> : (
          <DataTable
            aria-label="Members"
            columns={[
              { key: 'username', header: 'Member', isRowHeader: true },
              { key: 'email', header: 'Email' },
              { key: 'role', header: 'Role' },
            ]}
            rows={data.members.map((m) => ({ ...m, id: m.user_id }))}
            renderCell={(member, key) => {
              if (key === 'username') return member.username;
              if (key === 'email') return <span className="text-forest-muted">{member.email}</span>;
              return <span className="text-forest-muted">{member.member_role}</span>;
            }}
          />
        )
      )}

      {tab === 'certificates' && (
        data.certificates.length === 0 ? <EmptyState title="No certificates issued" /> : (
          <DataTable
            aria-label="Certificates"
            columns={[
              { key: 'number', header: 'Certificate', isRowHeader: true },
              { key: 'event', header: 'Event' },
              { key: 'participant', header: 'Participant' },
              { key: 'issued', header: 'Issued' },
            ]}
            rows={data.certificates.map((c) => ({ ...c, id: c.participation_id }))}
            renderCell={(cert, key) => {
              if (key === 'number') return cert.certificate_number;
              if (key === 'event') return <span className="text-forest-muted">{cert.event_name}</span>;
              if (key === 'participant') return <span className="text-forest-muted">{cert.participant_name}</span>;
              return <span className="text-forest-muted">{formatDateTime(cert.issued_at)}</span>;
            }}
          />
        )
      )}

      {tab === 'impact' && (
        <div className="grid gap-4 md:grid-cols-4">
          <StatTile label="Verified activities" value={String(data.impact.verified_activities)} />
          <StatTile label="Participants" value={String(data.impact.active_participants)} />
          <StatTile label="Certificates" value={String(data.impact.certificates_issued)} />
          <StatTile label="Metrics" value={String(data.impact.metrics.length)} />
        </div>
      )}

      {tab === 'audit' && <AuditList entries={data.audit} />}

      <ReasonDialog
        open={nextStatus !== null}
        title={nextStatus ? `${statusLabel(nextStatus)} this organization` : 'Review'}
        confirmLabel="Save"
        destructive={nextStatus === 'REJECTED' || nextStatus === 'SUSPENDED' || nextStatus === 'INACTIVE'}
        busy={busy}
        onClose={() => setNextStatus(null)}
        onConfirm={review}
      />
    </div>
  );
}
