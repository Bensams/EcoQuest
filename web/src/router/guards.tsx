import { type ReactNode } from 'react';
import { Navigate, Outlet, useLocation } from 'react-router-dom';
import { useAuth } from '../lib/auth';
import type { Role } from '../lib/types';

export function ProtectedRoute({ allow = [], children }: { allow?: Role[]; children?: ReactNode }) {
  const { user, loading } = useAuth();
  const location = useLocation();

  if (loading) {
    return (
      <div className="grid min-h-screen place-items-center bg-canvas text-sm text-forest-muted">
        Loading…
      </div>
    );
  }

  if (!user) {
    return <Navigate to="/login" state={{ from: location }} replace />;
  }

  if (allow.length > 0 && !allow.includes(user.role)) {
    return <Navigate to="/app/missions" replace />;
  }

  return children ? <>{children}</> : <Outlet />;
}