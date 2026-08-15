import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { ArrowRight } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { ButtonLink } from '../../components/ui/Button';
import { Card, StatTile } from '../../components/ui/Card';
import { PageHeader } from '../../components/ui/PageHeader';
import { useAuth } from '../../lib/auth';
import { formatDateRange, participationStatusTone } from '../../lib/format';
import type { Activity, Achievement, CommunityGoal, ImpactStats } from '../../lib/types';
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

const STAGE_LABELS = ['Polluted', 'Cleanup started', 'Recovering', 'Healthy ecosystem'];
const STAGE_ART = ['scene-0-polluted', 'scene-1-cleanup', 'scene-2-recovering', 'scene-3-healthy'];
// Keyed off the strings the WASM module returns, so a new tier there surfaces
// here as a missing image rather than silently showing the wrong biome.
const ENVIRONMENT_ART: Record<string, string> = {
  Coast: 'environment-coast',
  Ocean: 'environment-ocean',
  Forest: 'environment-forest',
  'Global Explorer': 'environment-global',
};
const CHARACTER_ART: Record<string, string> = {
  Beginner: 'character-beginner',
  Turtle: 'character-turtle',
  'Eco Guardian': 'character-guardian',
};

function Caption({ children }: { children: React.ReactNode }) {
  return (
    <span className="absolute bottom-2 left-2 rounded-full bg-white/85 px-2.5 py-0.5 text-xs font-medium text-forest">
      {children}
    </span>
  );
}

/** The restoration scene for the current stage, beside the animated biome. */
function RestorationScene({ stage, environment }: { stage: number; environment: string }) {
  const scene = STAGE_ART[stage] ?? STAGE_ART[0];
  const biome = ENVIRONMENT_ART[environment];
  return (
    <div className="grid gap-3 sm:grid-cols-3">
      <div className="relative overflow-hidden rounded-lg sm:col-span-2">
        <img
          src={`/art/${scene}.webp`}
          alt={`Your coastline: ${STAGE_LABELS[stage] ?? STAGE_LABELS[0]}`}
          className="h-40 w-full object-cover"
        />
        <Caption>{STAGE_LABELS[stage] ?? STAGE_LABELS[0]}</Caption>
      </div>
      {biome && (
        <div className="relative overflow-hidden rounded-lg">
          {/* Readers who ask for less motion get the still frame instead of the loop. */}
          <picture>
            <source media="(prefers-reduced-motion: reduce)" srcSet={`/art/${biome}.webp`} />
            <img src={`/art/${biome}.gif`} alt={environment} className="h-40 w-full object-cover" />
          </picture>
          <Caption>{environment}</Caption>
        </div>
      )}
    </div>
  );
}

export function DashboardPage() {
  const { user } = useAuth();
  const { data: myImpact } = useFetch<ImpactStats | null>(user ? '/api/impact/me' : null, user?.id);
  const { data: goal } = useFetch<CommunityGoal>('/api/impact/community-goal');
  const { data: activities } = useFetch<Activity[]>(user ? '/api/me/activities' : null, user?.id);
  const { data: achievements } = useFetch<Achievement[]>(user ? '/api/achievements/me' : null, user?.id);
  const game = useRestorationGame(user?.eco_points ?? 0);

  const upcomingActivities = activities?.filter(
    (a) => a.participation.status === 'REGISTERED' || a.participation.status === 'PENDING_VERIFICATION'
  ) ?? [];
  const pendingVerification = activities?.filter((a) => a.participation.status === 'PENDING_VERIFICATION') ?? [];
  // Earned first, so the card never leads with achievements still at 0.
  const recentAchievements = [...(achievements ?? [])]
    .sort((a, b) => Number(b.earned) - Number(a.earned))
    .slice(0, 3);

  return (
    <div>
      <PageHeader
        eyebrow="Welcome"
        title={user ? `Welcome back, ${user.username}` : 'Explore EcoQuest'}
        detail="Track your progress, upcoming missions, and community impact."
      />

      <section className="mb-8 grid grid-cols-2 gap-4 md:grid-cols-4">
        <StatTile label="Eco Points" value={String(user?.eco_points ?? 0)} />
        <StatTile
          label="Level"
          value={game ? String(game.level) : '—'}
        />
        <StatTile label="Verified activities" value={String(myImpact?.verified_activities ?? 0)} />
        <StatTile label="Certificates" value={String(myImpact?.certificates_issued ?? 0)} />
      </section>

      {game && (
        <Card className="mb-8">
          <RestorationScene stage={game.restoration_stage} environment={game.environment} />
          <div className="mt-4 flex items-center gap-3">
            {CHARACTER_ART[game.character] && (
              <img
                src={`/art/${CHARACTER_ART[game.character]}.webp`}
                alt={game.character}
                className="size-14 shrink-0 rounded-full object-cover ring-1 ring-sage-soft"
              />
            )}
            <div className="min-w-0 flex-1">
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
          </div>
        </Card>
      )}

      <div className="grid gap-6 md:grid-cols-2">
        <Card>
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-base font-semibold text-forest">Upcoming missions</h2>
            <ButtonLink to="/app/activities" variant="ghost" size="sm">
              View all <ArrowRight className="size-3.5" aria-hidden="true" />
            </ButtonLink>
          </div>
          {upcomingActivities.length === 0 ? (
            <p className="text-sm text-forest-muted">No upcoming missions. Join one from the missions page.</p>
          ) : (
            <ul className="space-y-3">
              {upcomingActivities.slice(0, 3).map((act) => (
                <li key={act.participation.id} className="flex items-center justify-between gap-3">
                  <Link to={`/app/missions/${act.event.id}`} className="flex-1 min-w-0">
                    <p className="font-medium text-forest truncate">{act.event.name}</p>
                    <p className="text-xs text-forest-muted">{formatDateRange(act.event.starts_at, act.event.ends_at)}</p>
                  </Link>
                  <Badge tone={participationStatusTone(act.participation.status)}>{act.participation.status}</Badge>
                </li>
              ))}
            </ul>
          )}
        </Card>

        <Card>
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-base font-semibold text-forest">Pending verification</h2>
            <ButtonLink to="/app/activities" variant="ghost" size="sm">
              View all <ArrowRight className="size-3.5" aria-hidden="true" />
            </ButtonLink>
          </div>
          {pendingVerification.length === 0 ? (
            <p className="text-sm text-forest-muted">Nothing pending. Check back after your next check-in.</p>
          ) : (
            <ul className="space-y-3">
              {pendingVerification.slice(0, 3).map((act) => (
                <li key={act.participation.id} className="flex items-center justify-between gap-3">
                  <Link to={`/app/missions/${act.event.id}`} className="flex-1 min-w-0">
                    <p className="font-medium text-forest truncate">{act.event.name}</p>
                    <p className="text-xs text-forest-muted">{formatDateRange(act.event.starts_at, act.event.ends_at)}</p>
                  </Link>
                  <Badge tone="warning">Pending</Badge>
                </li>
              ))}
            </ul>
          )}
        </Card>

        <Card>
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-base font-semibold text-forest">Recent achievements</h2>
            <ButtonLink to="/app/achievements" variant="ghost" size="sm">
              View all <ArrowRight className="size-3.5" aria-hidden="true" />
            </ButtonLink>
          </div>
          {recentAchievements.length === 0 ? (
            <p className="text-sm text-forest-muted">No achievements yet. Verified participation unlocks them.</p>
          ) : (
            <ul className="space-y-3">
              {recentAchievements.map((ach) => (
                <li key={`${ach.kind}-${ach.achievement_key}`} className="flex items-center justify-between gap-3">
                  <span className="text-sm text-forest">{ach.title || ach.achievement_key.replace(/_/g, ' ')}</span>
                  <Badge tone={ach.earned ? 'success' : 'muted'}>
                    {ach.earned ? 'Earned' : `${ach.progress} / ${ach.threshold}`}
                  </Badge>
                </li>
              ))}
            </ul>
          )}
        </Card>

        <Card>
          <div className="mb-4 flex items-center justify-between">
            <h2 className="text-base font-semibold text-forest">Community goal</h2>
            <ButtonLink to="/app/impact" variant="ghost" size="sm">
              View progress <ArrowRight className="size-3.5" aria-hidden="true" />
            </ButtonLink>
          </div>
          {goal ? (
            <>
              <p className="mb-2 text-sm text-forest-muted">{goal.name}</p>
              <p className="mb-2 text-sm text-forest">{goal.current_value} / {goal.target_value} {goal.unit}</p>
              <div className="h-2 overflow-hidden rounded-full bg-sage-soft">
                <div
                  className="h-full rounded-full bg-leaf"
                  style={{ width: `${Math.min(100, (goal.current_value / Math.max(1, goal.target_value)) * 100)}%` }}
                />
              </div>
            </>
          ) : (
            <p className="text-sm text-forest-muted">No active community goal.</p>
          )}
        </Card>
      </div>

      <div className="mt-8 flex justify-center">
        <ButtonLink to="/app/missions" variant="primary" size="md" className="w-full md:w-auto">
          <ArrowRight className="size-4" aria-hidden="true" /> Explore Missions
        </ButtonLink>
      </div>
    </div>
  );
}