import {
  Award,
  Bell,
  Building2,
  CalendarClock,
  Home,
  Leaf,
  LogOut,
  Menu,
  QrCode,
  ShieldCheck,
  Users,
  User,
  X,
} from 'lucide-react';
import { NavLink, useNavigate } from 'react-router-dom';
import { useAuth } from '../../lib/auth';
import type { OrganizationStatus } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';
import { cn } from '../../lib/utils';

const primaryNav = [
  { to: '/app/dashboard', label: 'Dashboard', icon: Home },
  { to: '/app/missions', label: 'Missions', icon: CalendarClock },
  { to: '/app/activities', label: 'My Activities', icon: Bell },
  { to: '/app/achievements', label: 'Achievements', icon: Award },
  { to: '/app/certificates', label: 'Certificates', icon: Leaf },
  { to: '/app/check-in', label: 'Check-in', icon: QrCode },
  { to: '/app/profile', label: 'Profile', icon: User },
];

const adminNav = [
  { to: '/admin', label: 'Admin dashboard', icon: ShieldCheck, end: true },
  { to: '/admin/organizations', label: 'Organizations', icon: Users },
  { to: '/admin/events', label: 'Events', icon: CalendarClock },
  { to: '/admin/users', label: 'Users', icon: Users },
];

export function SidebarContent({ onNavigate }: { onNavigate?: () => void }) {
  const { user } = useAuth();
  // Organization capability is ownership, not a role, so the nav is driven by
  // the caller's memberships rather than user.role.
  const { data: organizations } = useFetch<OrganizationStatus[]>(
    user ? '/api/organizations/me/status' : null,
    user?.id,
  );
  const organization = organizations?.[0];

  const NavItem = ({ to, label, icon: Icon, end }: { to: string; label: string; icon: typeof Home; end?: boolean }) => (
    <NavLink
      to={to}
      end={end}
      onClick={onNavigate}
      className={({ isActive }) =>
        cn(
          'flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors',
          isActive ? 'bg-sage-soft text-leaf' : 'text-forest-muted hover:bg-sage-soft hover:text-forest',
        )
      }
    >
      <Icon className="size-4 shrink-0" aria-hidden="true" />
      {label}
    </NavLink>
  );

  return (
    <div data-component="sidebar" className="flex h-full flex-col gap-6 overflow-y-auto p-4">
      <NavLink to="/app/dashboard" className="flex items-center gap-2 font-semibold text-forest">
        <span className="grid size-8 place-items-center rounded-lg bg-leaf text-white">
          <Leaf className="size-4" aria-hidden="true" />
        </span>
        EcoQuest
      </NavLink>

      <nav aria-label="Main" className="grid gap-1">
        {primaryNav.map((item) => (
          <NavItem key={item.to} {...item} />
        ))}
      </nav>

      <div>
        <p className="px-3 pb-2 text-xs font-medium uppercase tracking-wide text-forest-muted">Organization</p>
        <nav aria-label="Organization" className="grid gap-1">
          {organization ? (
            <>
              <NavItem to="/org" label="My organization" icon={Building2} end />
              {organization.verification_status === 'APPROVED' && (
                <NavItem
                  to={`/org/${organization.organization_id}/events`}
                  label="Manage events"
                  icon={CalendarClock}
                />
              )}
            </>
          ) : (
            <NavItem to="/org/create" label="Start an organization" icon={Building2} />
          )}
        </nav>
      </div>

      {user?.role === 'ADMIN' && (
        <div>
          <p className="px-3 pb-2 text-xs font-medium uppercase tracking-wide text-forest-muted">Administration</p>
          <nav aria-label="Administration" className="grid gap-1">
            {adminNav.map((item) => (
              <NavItem key={item.to} {...item} />
            ))}
          </nav>
        </div>
      )}
    </div>
  );
}

export function Sidebar() {
  return (
    <aside data-component="sidebar-container" className="hidden w-60 shrink-0 border-r border-sage bg-surface lg:block">
      <SidebarContent />
    </aside>
  );
}

export function MobileDrawer({ open, onClose }: { open: boolean; onClose: () => void }) {
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-40 lg:hidden" role="dialog" aria-modal="true" aria-label="Navigation">
      <div className="absolute inset-0 bg-forest/40" onClick={onClose} aria-hidden="true" />
      <div className="absolute inset-y-0 left-0 w-72 bg-surface shadow-xl transition-transform">
        <button
          className="absolute right-3 top-4 rounded-lg p-1.5 text-forest-muted hover:text-forest"
          onClick={onClose}
          aria-label="Close menu"
        >
          <X className="size-5" aria-hidden="true" />
        </button>
        <SidebarContent onNavigate={onClose} />
      </div>
    </div>
  );
}

export function Topbar({ onMenu }: { onMenu: () => void }) {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  const handleLogout = async () => {
    await logout();
    navigate('/login', { replace: true });
  };

  return (
    <header data-component="topbar" className="sticky top-0 z-30 border-b border-sage bg-surface/90 backdrop-blur">
      <div className="mx-auto flex h-14 max-w-6xl items-center gap-3 px-4">
        <button
          className="rounded-lg p-2 text-forest-muted hover:bg-sage-soft lg:hidden"
          onClick={onMenu}
          aria-label="Open menu"
        >
          <Menu className="size-5" aria-hidden="true" />
        </button>
        <div className="ml-auto flex items-center gap-3 text-sm text-forest-muted">
          {user && (
            <span>
              {user.username} · <strong className="font-semibold text-forest">{user.eco_points} pts</strong>
            </span>
          )}
          <button
            className="flex items-center gap-1.5 rounded-lg px-2 py-1.5 text-sm font-medium text-forest-muted transition-colors hover:bg-sage-soft hover:text-forest"
            onClick={() => void handleLogout()}
          >
            <LogOut className="size-4" aria-hidden="true" />
            Sign out
          </button>
        </div>
      </div>
    </header>
  );
}