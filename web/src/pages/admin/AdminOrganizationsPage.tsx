import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { ShieldCheck } from 'lucide-react';
import { AdminToolbar } from '../../components/admin/AdminToolbar';
import { ReasonDialog } from '../../components/admin/ReasonDialog';
import { Badge } from '../../components/ui/Badge';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { DataTable } from '../../components/ui/Table';
import { EmptyState, ErrorNote, LoadingState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { adminListPath, defaultAdminQuery, type AdminQuery } from '../../lib/adminQuery';
import { patch } from '../../lib/api';
import { statusLabel, verificationToneOf } from '../../lib/format';
import type { AdminOrganization, Page, VerificationStatus } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

type Action = { org: AdminOrganization; status: VerificationStatus; label: string; destructive?: boolean };

function actionsFor(org: AdminOrganization): Array<Omit<Action, 'org'>> {
  switch (org.verification_status) {
    case 'PENDING':
      return [
        { status: 'APPROVED', label: 'Approve' },
        { status: 'REJECTED', label: 'Reject', destructive: true },
      ];
    case 'APPROVED':
      return [
        { status: 'SUSPENDED', label: 'Suspend', destructive: true },
        { status: 'INACTIVE', label: 'Deactivate', destructive: true },
      ];
    case 'SUSPENDED':
      return [
        { status: 'APPROVED', label: 'Reactivate' },
        { status: 'INACTIVE', label: 'Deactivate', destructive: true },
      ];
    case 'REJECTED':
      return [{ status: 'PENDING', label: 'Reopen' }];
    case 'INACTIVE':
      return [{ status: 'APPROVED', label: 'Reactivate' }];
  }
}

export function AdminOrganizationsPage() {
  const [query, setQuery] = useState<AdminQuery>(defaultAdminQuery());
  const path = useMemo(() => adminListPath('/api/admin/organizations', query), [query]);
  const { data, error, loading, reload } = useFetch<Page<AdminOrganization>>(path, path);
  const [notice, setNotice] = useState<string | null>(null);
  const [action, setAction] = useState<Action | null>(null);
  const [busy, setBusy] = useState(false);

  const review = async (reason: string) => {
    if (!action) return;
    setNotice(null);
    setBusy(true);
    try {
      await patch(`/api/admin/organizations/${action.org.id}/status`, { status: action.status, reason });
      setNotice(`“${action.org.name}” is now ${statusLabel(action.status).toLowerCase()}.`);
      setAction(null);
      await reload();
    } catch (err) {
      setNotice(err instanceof Error ? err.message : 'Review action failed.');
    } finally {
      setBusy(false);
    }
  };

  const items = data?.items ?? [];

  return (
    <div>
      <PageHeader
        eyebrow="Administration"
        title="Organizations"
        detail="Search, review, suspend, and reactivate organizations. Every change needs a reason."
      />
      {notice && <div className="mb-4"><ErrorNote message={notice} /></div>}
      <AdminToolbar
        query={query}
        onChange={setQuery}
        total={data?.total ?? 0}
        statuses={[
          { value: 'PENDING', label: 'Pending' },
          { value: 'APPROVED', label: 'Approved' },
          { value: 'REJECTED', label: 'Rejected' },
          { value: 'SUSPENDED', label: 'Suspended' },
          { value: 'INACTIVE', label: 'Inactive' },
        ]}
      />
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}
      {loading && <LoadingState label="Loading organizations…" />}

      <Card>
        <h2 className="mb-4 flex items-center gap-2 text-sm font-semibold text-forest">
          <ShieldCheck className="size-4" aria-hidden="true" /> Organizations
        </h2>
        {!loading && items.length === 0 ? (
          <EmptyState title="No organizations match" detail="Try another search or status filter." />
        ) : items.length > 0 ? (
          <DataTable
            aria-label="Organizations"
            columns={[
              { key: 'org', header: 'Organization', isRowHeader: true },
              { key: 'owner', header: 'Owner' },
              { key: 'location', header: 'Location' },
              { key: 'events', header: 'Events' },
              { key: 'status', header: 'Status' },
              { key: 'review', header: 'Review' },
            ]}
            rows={items}
            renderCell={(org, key) => {
              switch (key as string) {
                case 'org':
                  return (
                    <Link to={`/admin/organizations/${org.id}`} className="font-medium text-forest hover:text-leaf">
                      {org.name}
                    </Link>
                  );
                case 'owner':
                  return <span className="text-forest-muted">{org.owner_username ?? '—'}</span>;
                case 'location':
                  return <span className="text-forest-muted">{org.location}</span>;
                case 'events':
                  return <span className="text-forest-muted">{org.event_count}</span>;
                case 'status':
                  return <Badge tone={verificationToneOf(org.verification_status)}>{statusLabel(org.verification_status)}</Badge>;
                case 'review':
                  return (
                    <div className="flex flex-wrap gap-2">
                      {actionsFor(org).map((item) => (
                        <Button
                          key={item.status}
                          size="sm"
                          variant={item.destructive ? 'destructive' : 'secondary'}
                          onClick={() => setAction({ org, ...item })}
                        >
                          {item.label}
                        </Button>
                      ))}
                    </div>
                  );
                default:
                  return null;
              }
            }}
          />
        ) : null}
      </Card>

      <ReasonDialog
        open={action !== null}
        title={action ? `${action.label} “${action.org.name}”` : 'Review organization'}
        description="This is recorded on the organization audit trail."
        confirmLabel={action?.label ?? 'Confirm'}
        destructive={action?.destructive}
        busy={busy}
        onClose={() => setAction(null)}
        onConfirm={review}
      />
    </div>
  );
}
