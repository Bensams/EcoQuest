import { useEffect, useRef, useState } from 'react';
import { Card } from '../../components/ui/Card';
import { EmptyState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { StatTile } from '../../components/ui/Card';
import { useAuth } from '../../lib/auth';
import type { CommunityGoal, ImpactStats } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

const GAME_WASM_URL = new URL('/game/ecoquest_game_wasm.js', import.meta.url).href;

type GameState = {
  level: number;
  progress_points: number;
  points_to_next_level: number;
  character: string;
  environment: string;
  restoration_stage: number;
};

type GameWasm = { game_state_json(points: number): string };

export function ImpactPage() {
  const { user } = useAuth();
  const { data: globalImpact } = useFetch<ImpactStats>('/api/impact');
  const { data: myImpact } = useFetch<ImpactStats | null>(user ? '/api/impact/me' : null, user?.id);
  const { data: goal } = useFetch<CommunityGoal>('/api/impact/community-goal');
  const game = useRestorationGame(user?.eco_points ?? 0);

  const metricRows = (stats: ImpactStats | null | undefined) =>
    stats?.metrics.map((m) => (
      <div key={`${m.metric}-${m.unit}`} className="flex items-center justify-between border-b border-sage py-1.5 text-sm last:border-0">
        <span className="capitalize text-forest-muted">{m.metric.replace(/_/g, ' ')}</span>
        <span className="font-medium text-forest">{m.value} {m.unit}</span>
      </div>
    ));

  return (
    <div>
      <PageHeader eyebrow="Your contribution" title="Impact" />
      {!user && <EmptyState title="Sign in to track your personal impact" />}

      <section className="mb-8 grid grid-cols-2 gap-4 md:grid-cols-4">
        <StatTile label="Eco Points" value={String(user?.eco_points ?? 0)} />
        <StatTile
          label="Level"
          value={game ? String(game.level) : '—'}
        />
        <StatTile label="Verified activities" value={String(myImpact?.verified_activities ?? globalImpact?.verified_activities ?? 0)} />
        <StatTile label="Certificates" value={String(myImpact?.certificates_issued ?? globalImpact?.certificates_issued ?? 0)} />
      </section>

      {game && (
        <Card className="mb-8">
          <RestorationScene stage={game.restoration_stage} />
          <div className="mt-4">
            <p className="mb-1 text-sm text-forest-muted">
              {game.character} · {game.environment} · {game.progress_points} pts
            </p>
            <div className="h-2 overflow-hidden rounded-full bg-sage-soft">
              <div
                className="h-full rounded-full bg-leaf"
                style={{
                  width: `${Math.min(100, (game.progress_points / (game.progress_points + game.points_to_next_level || 1)) * 100)}%`,
                }}
              />
            </div>
            <p className="mt-1 text-xs text-forest-muted">
              {game.points_to_next_level ? `${game.points_to_next_level} points to next level` : 'Maximum level reached'}
            </p>
          </div>
        </Card>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <TableTitle>My impact</TableTitle>
          {user ? (
            myImpact ? (
              <div>{metricRows(myImpact)}</div>
            ) : (
              <p className="text-sm text-forest-muted">No verified activities yet. Join a mission and earn points.</p>
            )
          ) : (
            <p className="text-sm text-forest-muted">Sign in to view personal verified impact.</p>
          )}
        </Card>
        <Card>
          <TableTitle>Platform impact</TableTitle>
          {globalImpact && (
            <div className="mb-4 space-y-1 text-sm text-forest-muted">
              <p>{globalImpact.active_participants} active participants</p>
              <p>{globalImpact.approved_organizations} approved organizations</p>
            </div>
          )}
          <div>{metricRows(globalImpact)}</div>
        </Card>
      </div>

      {goal && (
        <Card className="mt-8">
          <h2 className="mb-2 text-sm font-semibold text-forest">Community goal: {goal.name}</h2>
          <p className="mb-2 text-sm text-forest-muted">
            {goal.current_value} / {goal.target_value} {goal.unit}
          </p>
          <div className="h-2 overflow-hidden rounded-full bg-sage-soft">
            <div
              className="h-full rounded-full bg-leaf"
              style={{ width: `${Math.min(100, (goal.current_value / Math.max(1, goal.target_value)) * 100)}%` }}
            />
          </div>
        </Card>
      )}
    </div>
  );
}

function TableTitle({ children }: { children: React.ReactNode }) {
  return <h2 className="mb-3 text-sm font-semibold text-forest">{children}</h2>;
}

function RestorationScene({ stage }: { stage: number }) {
  const scenes = ['Polluted', 'Cleanup started', 'Recovering', 'Healthy ecosystem'];
  return (
    <div className="relative min-h-24 overflow-hidden rounded-lg bg-sky-100 p-4 text-2xl">
      <span className="absolute right-3 top-2" aria-hidden="true">☀</span>
      <span className="absolute bottom-3 left-1/2 -translate-x-1/2" aria-hidden="true">
        {stage === 3 ? '🌳 🐟 🦋' : stage === 2 ? '🌱 🐟' : stage === 1 ? '🗑️ 🌱' : '🗑️ 🛢️'}
      </span>
      <p className="absolute bottom-1 right-3 text-xs font-medium text-forest-muted">{scenes[stage]}</p>
    </div>
  );
}

function useRestorationGame(points: number): GameState | null {
  const wasmRef = useRef<GameWasm | null>(null);
  const [state, setState] = useState<GameState | null>(null);

  useEffect(() => {
    let disposed = false;
    import(/* @vite-ignore */ GAME_WASM_URL)
      .then(async (module) => {
        await module.default();
        wasmRef.current = module as GameWasm;
      })
      .catch(() => {
        // Game stays unavailable; the rest of the page still works.
      })
      .finally(() => {
        if (!disposed && wasmRef.current) {
          setState(JSON.parse(wasmRef.current.game_state_json(points)) as GameState);
        }
      });
    return () => {
      disposed = true;
    };
  }, [points]);

  return state;
}