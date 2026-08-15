import { Link } from 'react-router-dom';
import { CalendarClock, ShieldCheck, UserCog, Users } from 'lucide-react';
import { Card, StatTile } from '../../components/ui/Card';
import { EmptyState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { useAuth } from '../../lib/auth';
import type { AdminEvent, AdminOrganization, AdminUser, ImpactStats, Page } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

export function AdminHomePage() {
  const { user } = useAuth();
  const { data: impact } = useFetch<ImpactStats>('/api/impact');
  const { data: organizations } = useFetch<Page<AdminOrganization>>('/api/admin/organizations');
  const { data: pending } = useFetch<Page<AdminOrganization>>('/api/admin/organizations?status=PENDING&per_page=1');
  const { data: users } = useFetch<Page<AdminUser>>('/api/admin/users');
  const { data: events } = useFetch<Page<AdminEvent>>('/api/admin/events');

  if (!user) return <EmptyState title="Sign in to view the admin dashboard" />;

  const pendingOrgs = pending?.total ?? 0;
  const sections = [
    {
      to: '/admin/organizations',
      label: 'Organizations',
      detail: 'Approve, reject, or suspend organization applications.',
      icon: Users,
      count: organizations?.total ?? 0,
    },
    {
      to: '/admin/events',
      label: 'Events',
      detail: 'Review every event and cancel ones that should not run.',
      icon: CalendarClock,
      count: events?.total ?? 0,
    },
    {
      to: '/admin/users',
      label: 'Users',
      detail: 'Change roles and suspend or reinstate accounts.',
      icon: UserCog,
      count: users?.total ?? 0,
    },
  ];

  return (
    <div>
      <PageHeader eyebrow="Platform" title="Admin dashboard" detail="Moderate organizations, events, and accounts." />

      <section className="mb-8 grid grid-cols-2 gap-4 md:grid-cols-4">
        <StatTile label="Verified activities" value={String(impact?.verified_activities ?? 0)} />
        <StatTile label="Active participants" value={String(impact?.active_participants ?? 0)} />
        <StatTile label="Approved organizations" value={String(impact?.approved_organizations ?? 0)} />
        <StatTile label="Certificates issued" value={String(impact?.certificates_issued ?? 0)} />
      </section>

      <div className="grid gap-4 md:grid-cols-3">
        {sections.map((section) => {
          const Icon = section.icon;
          return (
            <Link key={section.to} to={section.to} className="block">
              <Card className="h-full transition-colors hover:border-leaf">
                <h2 className="mb-1 flex items-center gap-2 text-sm font-semibold text-forest">
                  <Icon className="size-4" aria-hidden="true" /> {section.label}
                </h2>
                <p className="mb-4 text-sm text-forest-muted">{section.detail}</p>
                <p className="text-2xl font-semibold text-forest">{section.count}</p>
              </Card>
            </Link>
          );
        })}
      </div>

      <Card className="mt-6">
        <h2 className="mb-1 flex items-center gap-2 text-sm font-semibold text-forest">
          <ShieldCheck className="size-4" aria-hidden="true" /> Pending review
        </h2>
        <p className="text-sm text-forest-muted">
          {pendingOrgs} organization{pendingOrgs === 1 ? '' : 's'} waiting for a decision.
        </p>
      </Card>
    </div>
  );
}
