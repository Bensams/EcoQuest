import { Link } from 'react-router-dom';
import { Award, Leaf, ShieldCheck, User } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { ButtonLink } from '../../components/ui/Button';
import { Card, StatTile } from '../../components/ui/Card';
import { EmptyState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { useAuth } from '../../lib/auth';
import { formatDateTime } from '../../lib/format';
import type { Achievement, Certificate, OrganizationStatus } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

export function ProfilePage() {
  const { user } = useAuth();
  const { data: orgs } = useFetch<OrganizationStatus[]>(
    user ? '/api/organizations/me/status' : null,
    user?.id,
  );
  const { data: achievements } = useFetch<Achievement[]>(
    user ? '/api/achievements/me' : null,
    user?.id,
  );
  const { data: certificates } = useFetch<Certificate[]>(
    user ? '/api/certificates/me' : null,
    user?.id,
  );

  if (!user) return <EmptyState title="Sign in to view your profile" />;

  return (
    <div>
      <PageHeader eyebrow="Profile" title={user.username} detail="Manage your account, linked wallets, and organization status." />

      <section className="mb-8 grid grid-cols-2 gap-4 md:grid-cols-4">
        <StatTile label="Eco Points" value={String(user.eco_points)} />
        <StatTile label="Role" value={user.role} />
        <StatTile label="Achievements" value={String(achievements?.length ?? 0)} />
        <StatTile label="Certificates" value={String(certificates?.length ?? 0)} />
      </section>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <h2 className="mb-4 text-sm font-semibold text-forest">Account</h2>
          <div className="flex items-center gap-4">
            <span className="grid size-14 place-items-center rounded-full bg-sage-soft text-forest">
              <User className="size-6" aria-hidden="true" />
            </span>
            <div className="min-w-0 flex-1">
              <p className="font-semibold text-forest">{user.username}</p>
              <p className="text-sm text-forest-muted break-all">{user.email}</p>
              <p className="mt-1 text-xs text-forest-muted">
                Joined {formatDateTime(user.created_at)}
              </p>
            </div>
          </div>

          <div className="mt-6 border-t border-sage pt-4">
            <h3 className="mb-2 text-xs font-medium uppercase tracking-wide text-forest-muted">Organization status</h3>
            {orgs && orgs.length > 0 ? (
              orgs.map((org) => (
                <div key={org.organization_id} className="mb-2 flex items-center justify-between">
                  <span className="text-sm text-forest">{org.name}</span>
                  <Badge tone={orgStatusTone(org.verification_status)}>{orgStatusLabel(org.verification_status)}</Badge>
                </div>
              ))
            ) : (
              <p className="text-sm text-forest-muted">No organization yet.</p>
            )}
            <div className="mt-4">
              {orgs && orgs.some((o) => o.verification_status === 'APPROVED') ? (
                <ButtonLink to="/org" variant="secondary" size="sm">
                  Manage organization
                </ButtonLink>
              ) : (
                <ButtonLink to="/org/create" variant="secondary" size="sm">
                  <Leaf className="size-4" aria-hidden="true" /> Create organization
                </ButtonLink>
              )}
            </div>
          </div>
        </Card>

        <div className="space-y-6">
          <Card>
            <h2 className="mb-4 text-sm font-semibold text-forest">Wallet</h2>
            {user.wallet_address ? (
              <div className="flex items-center justify-between gap-3">
                <code className="block text-xs font-mono text-forest-muted break-all">
                  {user.wallet_address}
                </code>
                <Badge tone="success">Linked</Badge>
              </div>
            ) : (
              <p className="text-sm text-forest-muted">No wallet linked.</p>
            )}
            <ButtonLink to="/app/wallet" variant="secondary" size="sm" className="mt-3 w-full">
              Manage wallet &amp; achievements
            </ButtonLink>
          </Card>

          <Card>
            <h2 className="mb-4 text-sm font-semibold text-forest">Recognition</h2>
            <div className="flex flex-wrap gap-3">
              <Link
                to="/app/achievements"
                className="inline-flex items-center gap-1.5 rounded-lg border border-sage bg-white px-3 py-2 text-sm font-medium text-forest transition-colors hover:bg-sage-soft"
              >
                <Award className="size-4" aria-hidden="true" /> Achievements
              </Link>
              <Link
                to="/app/certificates"
                className="inline-flex items-center gap-1.5 rounded-lg border border-sage bg-white px-3 py-2 text-sm font-medium text-forest transition-colors hover:bg-sage-soft"
              >
                <ShieldCheck className="size-4" aria-hidden="true" /> Certificates
              </Link>
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}

function orgStatusTone(status: string) {
  switch (status) {
    case 'APPROVED':
      return 'success';
    case 'PENDING':
      return 'warning';
    case 'REJECTED':
    case 'SUSPENDED':
      return 'destructive';
    default:
      return 'muted';
  }
}

function orgStatusLabel(status: string) {
  return (status.charAt(0) + status.slice(1).toLowerCase()).replace('_', ' ');
}
