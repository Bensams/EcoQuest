import { useState } from 'react';
import { UserCog } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { DataTable } from '../../components/ui/Table';
import { EmptyState, ErrorNote } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { patch } from '../../lib/api';
import { useAuth } from '../../lib/auth';
import type { AdminUser, Page, Role, UserStatus } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

const roleTone: Record<AdminUser['role'], 'muted' | 'success'> = {
  USER: 'muted',
  ADMIN: 'success',
};

const userStatusTone: Record<AdminUser['status'], 'success' | 'destructive' | 'muted'> = {
  ACTIVE: 'success',
  SUSPENDED: 'destructive',
  DELETED: 'muted',
};

export function AdminUsersPage() {
  const { user } = useAuth();
  const { data, error, loading, reload } = useFetch<Page<AdminUser>>('/api/admin/users');
  const users = data?.items ?? [];
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const setRole = async (target: AdminUser, role: Role) => {
    setNotice(null);
    setBusy(target.id);
    try {
      await patch(`/api/admin/users/${target.id}/role`, {
        role,
        reason: `Role changed to ${role} from the admin dashboard.`,
      });
      setNotice(`“${target.username}” is now ${role === 'ADMIN' ? 'an admin' : 'a user'}.`);
      await reload();
    } catch (err) {
      setNotice(err instanceof Error ? err.message : 'Could not change role.');
    } finally {
      setBusy(null);
    }
  };

  const setStatus = async (target: AdminUser, status: UserStatus) => {
    setNotice(null);
    setBusy(target.id);
    try {
      await patch(`/api/admin/users/${target.id}/status`, {
        status,
        reason: `Status changed to ${status} from the admin dashboard.`,
      });
      setNotice(`“${target.username}” status is now ${status.toLowerCase()}.`);
      await reload();
    } catch (err) {
      setNotice(err instanceof Error ? err.message : 'Could not change status.');
    } finally {
      setBusy(null);
    }
  };

  return (
    <div>
      <PageHeader
        eyebrow="Administration"
        title="Users"
        detail="Adjust roles and lifecycle status across the platform."
      />
      {notice && <div className="mb-4"><ErrorNote message={notice} /></div>}
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}
      {loading && <p className="mb-4 text-sm text-forest-muted">Loading users…</p>}

      <Card>
        <h2 className="mb-4 flex items-center gap-2 text-sm font-semibold text-forest">
          <UserCog className="size-4" aria-hidden="true" /> All users
        </h2>
        {!loading && (!users || users.length === 0) ? (
          <EmptyState title="No users yet" />
        ) : users && users.length > 0 ? (
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
              const self = u.id === user?.id;
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
                  return self ? (
                    <span className="text-xs text-forest-muted">This is you</span>
                  ) : (
                    <div className="flex flex-wrap gap-2">
                      {u.role !== 'ADMIN' ? (
                        <Button size="sm" variant="secondary" disabled={busy === u.id} onClick={() => void setRole(u, 'ADMIN')}>
                          Make admin
                        </Button>
                      ) : (
                        <Button size="sm" variant="secondary" disabled={busy === u.id} onClick={() => void setRole(u, 'USER')}>
                          Make user
                        </Button>
                      )}
                      {u.status === 'ACTIVE' ? (
                        <Button size="sm" variant="destructive" disabled={busy === u.id} onClick={() => void setStatus(u, 'SUSPENDED')}>
                          Suspend
                        </Button>
                      ) : u.status !== 'DELETED' ? (
                        <Button size="sm" variant="secondary" disabled={busy === u.id} onClick={() => void setStatus(u, 'ACTIVE')}>
                          Reinstate
                        </Button>
                      ) : null}
                    </div>
                  );
                default:
                  return null;
              }
            }}
          />
        ) : null}
      </Card>
    </div>
  );
}
