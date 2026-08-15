import { Compass } from 'lucide-react';
import { Link, useLocation } from 'react-router-dom';
import { Card } from '../../components/ui/Card';

/**
 * Terminal 404.
 *
 * This used to be a silent redirect to the missions list, which made a broken
 * or mistyped link indistinguishable from a working one — the user landed on a
 * normal page and had no way to tell the address had not been found.
 */
export function NotFoundPage() {
  const { pathname } = useLocation();
  return (
    <main className="min-h-screen bg-surface px-4 py-12">
      <div className="mx-auto max-w-md">
        <Card className="text-center">
          <span className="mx-auto mb-3 grid size-12 place-items-center rounded-full bg-sage-soft text-forest-muted">
            <Compass className="size-6" aria-hidden="true" />
          </span>
          <h1 className="mb-1 text-lg font-semibold text-forest">Page not found</h1>
          <p className="text-sm text-forest-muted">
            Nothing lives at <span className="break-all font-mono text-xs">{pathname}</span>.
          </p>
          <div className="mt-5 flex flex-wrap justify-center gap-2">
            <Link
              to="/app/missions"
              className="inline-flex items-center rounded-lg bg-forest px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-forest/90"
            >
              Browse missions
            </Link>
            <Link
              to="/app/dashboard"
              className="inline-flex items-center rounded-lg border border-sage bg-white px-4 py-2 text-sm font-medium text-forest transition-colors hover:bg-sage-soft"
            >
              Go to dashboard
            </Link>
          </div>
        </Card>
      </div>
    </main>
  );
}
