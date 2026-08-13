import { Building2, CalendarPlus } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { ButtonLink } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { EmptyState, PageHeader } from '../../components/ui/EmptyState';
import { useAuth } from '../../lib/auth';
import type { ImpactStats, OrganizationStatus } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

const verificationBadge: Record<string, { tone: 'green' | 'amber' | 'gray' | 'red'; label: string }> = {
  APPROVED: { tone: 'green', label: 'Approved' },
  PENDING: { tone: 'amber', label: 'Pending review' },
  REJECTED: { tone: 'red', label: 'Rejected' },
  SUSPENDED: { tone: 'red', label: 'Suspended' },
};

export function OrgHomePage() {
  const { user } = useAuth();
  const { data: memberships, loading, error } = useFetch<OrganizationStatus[]>(
    user ? '/api/organizations/me/status' : null,
    user?.id,
  );

  return (
    <div>
      <PageHeader eyebrow="Organization" title="My organization" />
      {!user && <EmptyState title="Sign in to view your organizations" />}
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}
      {loading && <p className="text-sm text-forest-muted">Loading your organizations…</p>}

      {!loading && memberships && memberships.length === 0 && (
        <EmptyState
          title="You are not part of an organization yet"
          detail="Organizations are reviewed and approved by the platform before members can run events."
        />
      )}

      <div className="grid gap-4 md:grid-cols-2">
        {(memberships ?? []).map((membership) => {
          const badge = verificationBadge[membership.verification_status];
          return (
            <Card key={membership.organization_id}>
              <div className="mb-4 flex items-start justify-between gap-3">
                <div className="flex items-center gap-2">
                  <span className="grid size-9 place-items-center rounded-lg bg-sage-soft text-leaf">
                    <Building2 className="size-5" aria-hidden="true" />
                  </span>
                  <div>
                    <h2 className="font-semibold text-forest">{membership.name}</h2>
                    <p className="text-xs text-forest-muted">{membership.member_role}</p>
                  </div>
                </div>
                {badge && <Badge tone={badge.tone}>{badge.label}</Badge>}
              </div>
              <OrgImpact organizationId={membership.organization_id} />
              <div className="mt-4">
                <ButtonLink
                  to={`/org/${membership.organization_id}/events`}
                  variant="secondary"
                  size="sm"
                >
                  <CalendarPlus className="size-4" aria-hidden="true" /> Manage events
                </ButtonLink>
              </div>
            </Card>
          );
        })}
      </div>
    </div>
  );
}

function OrgImpact({ organizationId }: { organizationId: string }) {
  const { data } = useFetch<ImpactStats>(`/api/impact/organizations/${organizationId}`);
  if (!data) return null;
  return (
    <div className="rounded-lg bg-sage-soft px-4 py-3">
      <p className="mb-1 text-xs font-medium uppercase tracking-wide text-forest-muted">Verified impact</p>
      <p className="text-sm text-forest">
        {data.verified_activities} verified activities · {data.active_participants} participants · {data.certificates_issued} certificates
      </p>
      {data.metrics.length > 0 && (
        <ul className="mt-2 space-y-0.5 text-xs text-forest-muted">
          {data.metrics.map((m) => (
            <li key={`${m.metric}-${m.unit}`}>
              {m.value} {m.unit} {m.metric.replace(/_/g, ' ')}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}