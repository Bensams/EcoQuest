import { Award } from 'lucide-react';
import { Link } from 'react-router-dom';
import { Badge } from '../../components/ui/Badge';
import { Card, StatTile } from '../../components/ui/Card';
import { EmptyState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { useAuth } from '../../lib/auth';
import type { Achievement } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

function statusTone(status: string) {
  if (status.includes('ELIGIBLE') || status.includes('MINTED')) return 'success';
  if (status.includes('FAILED') || status.includes('REVOKED')) return 'destructive';
  return 'muted';
}

export function AchievementsPage() {
  const { user } = useAuth();
  const { data: achievements, loading, error } = useFetch<Achievement[]>(
    user ? '/api/achievements/me' : null,
    user?.id,
  );

  if (!user) return <EmptyState title="Sign in to view your achievements" detail="Verified participation unlocks platform achievements." />;

  const minted = achievements?.filter((a) => a.status.includes('MINTED')).length ?? 0;
  const eligible = achievements?.filter((a) => a.status.includes('ELIGIBLE')).length ?? 0;

  return (
    <div>
      <PageHeader eyebrow="Achievements" title="Your achievements" detail="On-chain and platform recognitions earned through verified participation." />
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}

      <section className="mb-8 grid grid-cols-2 gap-4 md:grid-cols-4">
        <StatTile label="Total" value={String(achievements?.length ?? 0)} />
        <StatTile label="Minted" value={String(minted)} />
        <StatTile label="Eligible" value={String(eligible)} />
      </section>

      {!loading && !error && (!achievements || achievements.length === 0) && (
        <EmptyState title="No achievements yet" detail="Earn them by completing and verifying participations." />
      )}

      {loading && <p className="text-sm text-forest-muted">Loading achievements…</p>}

      {!loading && !error && achievements && (
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {achievements.map((item) => (
            <Card key={item.achievement_key} className="flex gap-3 items-start">
              <span className="grid size-9 shrink-0 place-items-center rounded-lg bg-sage-soft text-amber">
                <Award className="size-5" aria-hidden="true" />
              </span>
              <div className="flex-1 min-w-0">
                <p className="font-medium text-forest">{item.achievement_key.replace(/_/g, ' ')}</p>
                {item.mint_identifier && (
                  <p className="text-xs text-forest-muted break-all">Contract {item.mint_identifier}</p>
                )}
                {item.transaction_signature && (
                  <p className="text-xs text-forest-muted break-all">Tx {item.transaction_signature}</p>
                )}
              </div>
              <Badge tone={statusTone(item.status)} className="ml-auto">{item.status}</Badge>
            </Card>
          ))}
        </div>
      )}

      {user && (
        <Card className="mt-8">
          <h2 className="mb-2 text-sm font-semibold text-forest">Link a wallet</h2>
          <p className="mb-4 text-sm text-forest-muted">
            Linking a wallet proves ownership of on-chain achievements. Signing only verifies you control the key — it never moves funds.
          </p>
          <Link
            to="/app/profile"
            className="inline-flex items-center gap-1.5 rounded-lg border border-sage bg-white px-4 py-2 text-sm font-medium text-forest transition-colors hover:bg-sage-soft"
          >
            Manage wallet &amp; profile
          </Link>
        </Card>
      )}
    </div>
  );
}
