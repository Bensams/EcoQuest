import { Leaf } from 'lucide-react';
import { NavLink, useNavigate } from 'react-router-dom';
import { useAuth } from '../../lib/auth';
import { Button } from '../ui/Button';

const navItems = [
  { to: '/app/missions', label: 'Missions', roles: ['PLAYER', 'ORGANIZATION_MEMBER', 'ADMIN'] },
  { to: '/app/impact', label: 'Impact', roles: ['PLAYER', 'ORGANIZATION_MEMBER', 'ADMIN'] },
  { to: '/app/check-in', label: 'Check-in', roles: ['PLAYER', 'ORGANIZATION_MEMBER', 'ADMIN'] },
  { to: '/app/wallet', label: 'Wallet', roles: ['PLAYER', 'ORGANIZATION_MEMBER', 'ADMIN'] },
  { to: '/org', label: 'My organization', roles: ['ORGANIZATION_MEMBER', 'ADMIN'] },
  { to: '/admin', label: 'Admin', roles: ['ADMIN'] },
];

export function TopNav() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();

  if (!user) return null;

  const visible = navItems.filter((item) => item.roles.includes(user.role));

  const handleLogout = async () => {
    await logout();
    navigate('/login', { replace: true });
  };

  return (
    <header className="sticky top-0 z-10 border-b border-sage bg-white/90 backdrop-blur">
      <div className="mx-auto flex h-14 max-w-6xl items-center gap-6 px-4">
        <NavLink to="/app/missions" className="flex items-center gap-2 font-semibold text-forest">
          <span className="grid size-8 place-items-center rounded-lg bg-leaf text-white">
            <Leaf className="size-4" aria-hidden="true" />
          </span>
          EcoQuest
        </NavLink>
        <nav className="flex flex-1 items-center gap-1">
          {visible.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              className={({ isActive }) =>
                `rounded-lg px-3 py-1.5 text-sm font-medium transition-colors ${
                  isActive ? 'bg-sage-soft text-leaf' : 'text-forest-muted hover:text-forest'
                }`
              }
            >
              {item.label}
            </NavLink>
          ))}
        </nav>
        <div className="flex items-center gap-3">
          <span className="text-sm text-forest-muted">
            {user.username} · {user.eco_points} pts
          </span>
          <Button variant="ghost" size="sm" onClick={() => void handleLogout()}>
            Sign out
          </Button>
        </div>
      </div>
    </header>
  );
}