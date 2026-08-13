import { Building2, Plus } from 'lucide-react';
import { Badge, type BadgeTone } from '../../components/ui/Badge';
import { ButtonLink } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { EmptyState, LoadingState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { useAuth } from '../../lib/auth';
import type { ImpactStats, OrganizationStatus } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

const statusCopy: Record<string, { label: string; detail: string }> = {
  PENDING: {
    label: 'Pending review',
    detail: 'Your organization will be reviewed by an EcoQuest administrator. Once approved, you become its owner and gain access to event management.',
  },
  APPROVED: {
    label: 'Approved',
    detail: 'Your organization is approved. You can create and publish events.',
  },
  REJECTED: {
    label: 'Rejected',
    detail: 'Your organization application was not approved. Contact an administrator for details.',
  },
  SUSPENDED: {
    label: 'Suspended',
    detail: 'Your organization is suspended. Event management is paused until it is reviewed.',
  },
};

const badgeTone: Record<string, BadgeTone> = {
  PENDING: 'warning',
  APPROVED: 'success',
  REJECTED: 'destructive',
  SUSPENDED: 'destructive',
};

export function OrgHomePage() {
  const { user } = useAuth();
  const { data: organizations, loading, error } = useFetch<OrganizationStatus[]>(
    user ? '/api/organizations/me/status' : null,
    user?.id,
  );

  return (
    <div>
      <PageHeader
        eyebrow="Organization"
        title="My organization"
        action={
          <ButtonLink to="/org/create" variant="secondary" size="sm">
            <Plus className="size-4" aria-hidden="true" />
            Create organization
          </ButtonLink>
        }
      />
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}
      {loading && <LoadingState label="Loading your organization…" />}
      {!loading && !error && organizations && organizations.length === 0 && (
        <EmptyState
          title="You are not part of an organization yet"
          detail="Create an application and an EcoQuest administrator will review it. Once approved, you become the owner and can run events."
          action={
            <ButtonLink to="/org/create" variant="secondary" size="sm">
              Create organization
            </ButtonLink>
          }
        />
      )}
      <div className="grid gap-4 md:grid-cols-2">
        {(organizations ?? []).map((org) => {
          const copy = statusCopy[org.verification_status] ?? statusCopy.PENDING;
          return (
            <Card key={org.organization_id} className="flex flex-col gap-4">
              <div className="flex items-start justify-between gap-3">
                <div className="flex items-center gap-2">
                  <span className="grid size-9 place-items-center rounded-lg bg-sage-soft text-leaf">
                    <Building2 className="size-5" aria-hidden="true" />
                  </span>
                  <h2 className="font-semibold text-forest">{org.name}</h2>
                </div>
                <Badge tone={badgeTone[org.verification_status]}>
                  {copy.label}
                </Badge>
              </div>
              <p className="text-sm text-forest-muted">{copy.detail}</p>
              {org.verification_status === 'APPROVED' && (
                <>
                  <OrgImpact organizationId={org.organization_id} />
                  <div className="mt-auto">
                    <ButtonLink to={`/org/${org.organization_id}/events`} variant="secondary" size="sm">
                      Manage events
                    </ButtonLink>
                  </div>
                </>
              )}
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