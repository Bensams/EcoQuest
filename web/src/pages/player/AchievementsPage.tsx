import { Award, Link2 } from 'lucide-react';
import { Link } from 'react-router-dom';
import { Badge } from '../../components/ui/Badge';
import type { BadgeTone } from '../../components/ui/Badge';
import { Card, StatTile } from '../../components/ui/Card';
import { EmptyState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { useAuth } from '../../lib/auth';
import type { Achievement } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

/** Mint statuses only appear on on-chain achievements. */
function statusTone(item: Achievement): BadgeTone {
  if (item.status === 'MINTED') return 'success';
  if (item.status === 'FAILED') return 'destructive';
  if (!item.earned) return 'muted';
  return item.kind === 'ONCHAIN' ? 'info' : 'success';
}

function statusLabel(item: Achievement) {
  if (item.kind === 'ONCHAIN') return item.status.replace(/_/g, ' ');
  return item.earned ? 'Earned' : `${item.progress} / ${item.threshold}`;
}

function AchievementCard({ item }: { item: Achievement }) {
  const percent = Math.min(100, (item.progress / Math.max(1, item.threshold)) * 100);
  return (
    <Card className={item.earned ? undefined : 'opacity-70'}>
      <div className="flex items-start gap-3">
        <span
          className={`grid size-9 shrink-0 place-items-center rounded-lg ${
            item.earned ? 'bg-amber-soft text-amber' : 'bg-sage-soft text-forest-muted'
          }`}
        >
          <Award className="size-5" aria-hidden="true" />
        </span>
        <div className="min-w-0 flex-1">
          <p className="font-medium text-forest">{item.title || item.achievement_key.replace(/_/g, ' ')}</p>
          <p className="text-xs text-forest-muted">{item.description}</p>
        </div>
        <Badge tone={statusTone(item)}>{statusLabel(item)}</Badge>
      </div>

      {!item.earned && item.kind === 'PLATFORM' && (
        <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-sage-soft">
          <div className="h-full rounded-full bg-leaf" style={{ width: `${percent}%` }} />
        </div>
      )}

      {item.mint_identifier && (
        <p className="mt-3 break-all text-xs text-forest-muted">Mint {item.mint_identifier}</p>
      )}
      {item.transaction_signature && (
        <p className="mt-1 break-all text-xs text-forest-muted">
          Tx {item.transaction_signature}
          {item.explorer_url && (
            <a
              href={item.explorer_url}
              target="_blank"
              rel="noreferrer"
              className="ml-1.5 inline-flex items-center gap-1 text-forest underline"
            >
              <Link2 className="size-3" aria-hidden="true" /> Explorer
            </a>
          )}
        </p>
      )}
      {item.kind === 'ONCHAIN' && item.earned && !item.wallet_address && (
        <p className="mt-3 text-xs text-forest-muted">
          Link a wallet to publish this achievement on chain.
        </p>
      )}
    </Card>
  );
}

export function AchievementsPage() {
  const { user } = useAuth();
  const { data: achievements, loading, error } = useFetch<Achievement[]>(
    user ? '/api/achievements/me' : null,
    user?.id,
  );

  if (!user)
    return (
      <EmptyState
        title="Sign in to view your achievements"
        detail="Verified participation unlocks platform achievements."
      />
    );

  const earned = achievements?.filter((a) => a.earned) ?? [];
  const inProgress = achievements?.filter((a) => !a.earned) ?? [];
  const minted = achievements?.filter((a) => a.status === 'MINTED').length ?? 0;

  return (
    <div>
      <PageHeader
        eyebrow="Achievements"
        title="Your achievements"
        detail="On-chain and platform recognitions earned through verified participation."
      />
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}

      <section className="mb-8 grid grid-cols-2 gap-4 md:grid-cols-4">
        <StatTile label="Earned" value={String(earned.length)} />
        <StatTile label="In progress" value={String(inProgress.length)} />
        <StatTile label="Minted on chain" value={String(minted)} />
      </section>

      {loading && <p className="text-sm text-forest-muted">Loading achievements…</p>}

      {!loading && !error && achievements && (
        <>
          <h2 className="mb-3 text-sm font-semibold text-forest">Earned</h2>
          {earned.length === 0 ? (
            <EmptyState
              title="No achievements yet"
              detail="Check in to a mission and have the organizer verify you to earn your first."
            />
          ) : (
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {earned.map((item) => (
                <AchievementCard key={`${item.kind}-${item.achievement_key}`} item={item} />
              ))}
            </div>
          )}

          {inProgress.length > 0 && (
            <>
              <h2 className="mb-3 mt-8 text-sm font-semibold text-forest">In progress</h2>
              <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                {inProgress.map((item) => (
                  <AchievementCard key={`${item.kind}-${item.achievement_key}`} item={item} />
                ))}
              </div>
            </>
          )}
        </>
      )}

      <Card className="mt-8">
        <h2 className="mb-2 text-sm font-semibold text-forest">Link a wallet</h2>
        <p className="mb-4 text-sm text-forest-muted">
          Linking a wallet publishes your on-chain achievements. Signing only verifies you control
          the key — it never moves funds.
        </p>
        <Link
          to="/app/profile"
          className="inline-flex items-center gap-1.5 rounded-lg border border-sage bg-white px-4 py-2 text-sm font-medium text-forest transition-colors hover:bg-sage-soft"
        >
          Manage wallet &amp; profile
        </Link>
      </Card>
    </div>
  );
}
