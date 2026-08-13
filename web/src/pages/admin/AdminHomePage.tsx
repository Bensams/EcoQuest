import { useState } from 'react';
import { ShieldCheck, UserCog } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Button } from '../../components/ui/Button';
import { Card, StatTile } from '../../components/ui/Card';
import { DataTable } from '../../components/ui/Table';
import { EmptyState, ErrorNote } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { patch } from '../../lib/api';
import { useAuth } from '../../lib/auth';
import { eventStatusTone } from '../../lib/format';
import type { AdminEvent, AdminOrganization, AdminUser, ImpactStats } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

const orgTone: Record<AdminOrganization['verification_status'], 'success' | 'warning' | 'destructive' | 'destructive'> = {
  APPROVED: 'success',
  PENDING: 'warning',
  REJECTED: 'destructive',
  SUSPENDED: 'destructive',
};

const roleTone: Record<AdminUser['role'], 'muted' | 'success'> = {
  USER: 'muted',
  ADMIN: 'success',
};

const userStatusTone: Record<AdminUser['status'], 'success' | 'destructive' | 'muted'> = {
  ACTIVE: 'success',
  SUSPENDED: 'destructive',
  DELETED: 'muted',
};

export function AdminHomePage() {
  const { user } = useAuth();
  const { data: impact } = useFetch<ImpactStats>('/api/impact');
  const { data: organizations, reload: reloadOrgs } = useFetch<AdminOrganization[]>('/api/admin/organizations');
  const { data: users, reload: reloadUsers } = useFetch<AdminUser[]>('/api/admin/users');
  const { data: events } = useFetch<AdminEvent[]>('/api/admin/events');
  const [notice, setNotice] = useState<string | null>(null);

  if (!user) return <EmptyState title="Sign in to view the admin dashboard" />;

  const review = async (org: AdminOrganization, status: 'APPROVED' | 'REJECTED' | 'SUSPENDED') => {
    setNotice(null);
    try {
      await patch(`/api/admin/organizations/${org.id}/status`, { status });
      setNotice(`“${org.name}” is now ${status.toLowerCase()}.`);
      await reloadOrgs();
    } catch (err) {
      setNotice(err instanceof Error ? err.message : 'Review action failed.');
    }
  };

  const setRole = async (target: AdminUser, role: AdminUser['role']) => {
    setNotice(null);
    try {
      await patch(`/api/admin/users/${target.id}/role`, { role });
      setNotice(`“${target.username}” is now ${role === 'ADMIN' ? 'an admin' : 'a user'}.`);
      await reloadUsers();
    } catch (err) {
      setNotice(err instanceof Error ? err.message : 'Could not change role.');
    }
  };

  const setStatus = async (target: AdminUser, status: AdminUser['status']) => {
    setNotice(null);
    try {
      await patch(`/api/admin/users/${target.id}/status`, { status });
      setNotice(`“${target.username}” status is now ${status.toLowerCase()}.`);
      await reloadUsers();
    } catch (err) {
      setNotice(err instanceof Error ? err.message : 'Could not change status.');
    }
  };

  return (
    <div>
      <PageHeader eyebrow="Platform" title="Admin dashboard" />
      {notice && <div className="mb-4"><ErrorNote message={notice} /></div>}

      <section className="mb-8 grid grid-cols-2 gap-4 md:grid-cols-4">
        <StatTile label="Verified activities" value={String(impact?.verified_activities ?? 0)} />
        <StatTile label="Active participants" value={String(impact?.active_participants ?? 0)} />
        <StatTile label="Approved organizations" value={String(impact?.approved_organizations ?? 0)} />
        <StatTile label="Certificates issued" value={String(impact?.certificates_issued ?? 0)} />
      </section>

      <Card className="mb-6">
        <h2 className="mb-1 flex items-center gap-2 text-sm font-semibold text-forest">
          <ShieldCheck className="size-4" aria-hidden="true" /> Organizations
        </h2>
        <p className="mb-4 text-sm text-forest-muted">Review applications so owners can publish events.</p>
        {!organizations || organizations.length === 0 ? (
          <p className="text-sm text-forest-muted">No organizations yet.</p>
        ) : (
          <DataTable
            aria-label="Organizations"
            columns={[
              { key: 'org', header: 'Organization', isRowHeader: true },
              { key: 'owner', header: 'Owner' },
              { key: 'type', header: 'Type' },
              { key: 'events', header: 'Events' },
              { key: 'status', header: 'Status' },
              { key: 'review', header: 'Review' },
            ]}
            rows={organizations}
            renderCell={(org, key) => {
              switch (key as string) {
                case 'org':
                  return <span className="font-medium text-forest">{org.name}</span>;
                case 'owner':
                  return <span className="text-forest-muted">{org.owner_username ?? '—'}</span>;
                case 'type':
                  return <span className="text-forest-muted">{org.organization_type.toLowerCase()}</span>;
                case 'events':
                  return <span className="text-forest-muted">{org.event_count}</span>;
                case 'status':
                  return <Badge tone={orgTone[org.verification_status]}>{org.verification_status}</Badge>;
                case 'review':
                  return (
                    <div className="flex flex-wrap gap-2">
                      {org.verification_status !== 'APPROVED' && (
                        <Button size="sm" variant="secondary" onClick={() => void review(org, 'APPROVED')}>Approve</Button>
                      )}
                      {org.verification_status !== 'REJECTED' && (
                        <Button size="sm" variant="destructive" onClick={() => void review(org, 'REJECTED')}>Reject</Button>
                      )}
                      {org.verification_status === 'APPROVED' && (
                        <Button size="sm" variant="destructive" onClick={() => void review(org, 'SUSPENDED')}>Suspend</Button>
                      )}
                    </div>
                  );
                default:
                  return null;
              }
            }}
          />
        )}
      </Card>

      <Card className="mb-6">
        <h2 className="mb-1 flex items-center gap-2 text-sm font-semibold text-forest">
          <ShieldCheck className="size-4" aria-hidden="true" /> Events
        </h2>
        <p className="mb-4 text-sm text-forest-muted">Moderation overview. Open an event to cancel it.</p>
        {!events || events.length === 0 ? (
          <p className="text-sm text-forest-muted">No events yet.</p>
        ) : (
          <DataTable
            aria-label="Events"
            columns={[
              { key: 'event', header: 'Event', isRowHeader: true },
              { key: 'org', header: 'Organization' },
              { key: 'status', header: 'Status' },
              { key: 'participants', header: 'Participants' },
              { key: 'start', header: 'Start' },
            ]}
            rows={events}
            renderCell={(ev, key) => {
              switch (key as string) {
                case 'event':
                  return <span className="font-medium text-forest">{ev.name}</span>;
                case 'org':
                  return <span className="text-forest-muted">{ev.organization_name}</span>;
                case 'status':
                  return <Badge tone={eventStatusTone(ev.status)}>{ev.status}</Badge>;
                case 'participants':
                  return <span className="text-forest-muted">{ev.registered_count}</span>;
                case 'start':
                  return <span className="text-forest-muted">{new Date(ev.starts_at).toLocaleDateString()}</span>;
                default:
                  return null;
              }
            }}
          />
        )}
      </Card>

      <Card>
        <h2 className="mb-1 flex items-center gap-2 text-sm font-semibold text-forest">
          <UserCog className="size-4" aria-hidden="true" /> Users
        </h2>
        <p className="mb-4 text-sm text-forest-muted">Adjust roles and lifecycle status across the platform.</p>
        {!users || users.length === 0 ? (
          <p className="text-sm text-forest-muted">No users yet.</p>
        ) : (
          <DataTable
            aria-label="Users"
            columns={[
              { key: 'username', header: 'Username', isRowHeader: true },
              { key: 'email', header: 'Email' },
              { key: 'points', header: 'Points' },
              { key: 'role', header: 'Role' },
              { key: 'status', header: 'Status' },
              { key: 'actions', header: 'Actions' },
            ]}
            rows={users}
            renderCell={(u, key) => {
              switch (key as string) {
                case 'username':
                  return <span className="font-medium text-forest">{u.username}</span>;
                case 'email':
                  return <span className="text-forest-muted">{u.email}</span>;
                case 'points':
                  return <span className="text-forest-muted">{u.eco_points}</span>;
                case 'role':
                  return <Badge tone={roleTone[u.role]}>{u.role}</Badge>;
                case 'status':
                  return <Badge tone={userStatusTone[u.status]}>{u.status}</Badge>;
                case 'actions':
                  return (
                    <div className="flex flex-wrap gap-2">
                      {u.role !== 'ADMIN' ? (
                        <Button size="sm" variant="secondary" onClick={() => void setRole(u, 'ADMIN')}>
                          Make admin
                        </Button>
                      ) : (
                        <Button size="sm" variant="secondary" onClick={() => void setRole(u, 'USER')}>
                          Make user
                        </Button>
                      )}
                      {u.status === 'ACTIVE' ? (
                        <Button size="sm" variant="destructive" onClick={() => void setStatus(u, 'SUSPENDED')}>
                          Suspend
                        </Button>
                      ) : (
                        <Button size="sm" variant="secondary" onClick={() => void setStatus(u, 'ACTIVE')}>
                          Reinstate
                        </Button>
                      )}
                    </div>
                  );
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