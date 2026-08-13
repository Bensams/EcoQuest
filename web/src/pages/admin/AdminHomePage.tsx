import { useState } from 'react';
import { ShieldCheck, UserCog } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Button } from '../../components/ui/Button';
import { Card, StatTile } from '../../components/ui/Card';
import { EmptyState, ErrorNote, PageHeader } from '../../components/ui/EmptyState';
import { patch } from '../../lib/api';
import { useAuth } from '../../lib/auth';
import type { AdminOrganization, AdminUser, ImpactStats } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

const orgTone: Record<AdminOrganization['verification_status'], 'green' | 'amber' | 'gray' | 'red'> = {
  APPROVED: 'green',
  PENDING: 'amber',
  REJECTED: 'red',
  SUSPENDED: 'red',
};

const roleTone: Record<AdminUser['role'], 'gray' | 'blue' | 'green'> = {
  PLAYER: 'gray',
  ORGANIZATION_MEMBER: 'blue',
  ADMIN: 'green',
};

const userStatusTone: Record<AdminUser['status'], 'green' | 'red' | 'gray'> = {
  ACTIVE: 'green',
  SUSPENDED: 'red',
  DELETED: 'gray',
};

export function AdminHomePage() {
  const { user } = useAuth();
  const { data: impact } = useFetch<ImpactStats>('/api/impact');
  const { data: organizations, reload: reloadOrgs } = useFetch<AdminOrganization[]>('/api/admin/organizations');
  const { data: users, reload: reloadUsers } = useFetch<AdminUser[]>('/api/admin/users');
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
      setNotice(`“${target.username}” is now ${role}.`);
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
        <p className="mb-4 text-sm text-forest-muted">Review applications so organizations can publish events.</p>
        {!organizations || organizations.length === 0 ? (
          <p className="text-sm text-forest-muted">No organizations yet.</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead>
                <tr className="border-b border-sage text-xs uppercase tracking-wide text-forest-muted">
                  <th className="py-2 pr-4 font-medium">Organization</th>
                  <th className="py-2 pr-4 font-medium">Type</th>
                  <th className="py-2 pr-4 font-medium">Location</th>
                  <th className="py-2 pr-4 font-medium">Members</th>
                  <th className="py-2 pr-4 font-medium">Status</th>
                  <th className="py-2 font-medium">Review</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-sage">
                {organizations.map((org) => (
                  <tr key={org.id}>
                    <td className="py-2.5 pr-4 font-medium text-forest">{org.name}</td>
                    <td className="py-2.5 pr-4 text-forest-muted">{org.organization_type.toLowerCase()}</td>
                    <td className="py-2.5 pr-4 text-forest-muted">{org.location}</td>
                    <td className="py-2.5 pr-4 text-forest-muted">{org.member_count}</td>
                    <td className="py-2.5 pr-4"><Badge tone={orgTone[org.verification_status]}>{org.verification_status}</Badge></td>
                    <td className="py-2.5">
                      {org.verification_status !== 'APPROVED' && (
                        <Button size="sm" variant="secondary" onClick={() => void review(org, 'APPROVED')}>Approve</Button>
                      )}
                      {org.verification_status !== 'REJECTED' && (
                        <Button size="sm" variant="destructive" onClick={() => void review(org, 'REJECTED')}>Reject</Button>
                      )}
                      {org.verification_status === 'APPROVED' && (
                        <Button size="sm" variant="destructive" onClick={() => void review(org, 'SUSPENDED')}>Suspend</Button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
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
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead>
                <tr className="border-b border-sage text-xs uppercase tracking-wide text-forest-muted">
                  <th className="py-2 pr-4 font-medium">Username</th>
                  <th className="py-2 pr-4 font-medium">Email</th>
                  <th className="py-2 pr-4 font-medium">Points</th>
                  <th className="py-2 pr-4 font-medium">Role</th>
                  <th className="py-2 pr-4 font-medium">Status</th>
                  <th className="py-2 font-medium">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-sage">
                {users.map((u) => (
                  <tr key={u.id}>
                    <td className="py-2.5 pr-4 font-medium text-forest">{u.username}</td>
                    <td className="py-2.5 pr-4 text-forest-muted">{u.email}</td>
                    <td className="py-2.5 pr-4 text-forest-muted">{u.eco_points}</td>
                    <td className="py-2.5 pr-4"><Badge tone={roleTone[u.role]}>{u.role}</Badge></td>
                    <td className="py-2.5 pr-4"><Badge tone={userStatusTone[u.status]}>{u.status}</Badge></td>
                    <td className="py-2.5">
                      <div className="flex flex-wrap gap-2">
                        {u.role !== 'ADMIN' && (
                          <Button size="sm" variant="secondary" onClick={() => void setRole(u, 'ADMIN')}>
                            Make admin
                          </Button>
                        )}
                        {u.role !== 'ORGANIZATION_MEMBER' && u.role !== 'ADMIN' && (
                          <Button size="sm" variant="secondary" onClick={() => void setRole(u, 'ORGANIZATION_MEMBER')}>
                            Make organizer
                          </Button>
                        )}
                        {u.role !== 'PLAYER' && (
                          <Button size="sm" variant="secondary" onClick={() => void setRole(u, 'PLAYER')}>
                            Make player
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