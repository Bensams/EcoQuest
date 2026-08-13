import { useState } from 'react';
import { Outlet } from 'react-router-dom';
import { MobileDrawer, Sidebar, Topbar } from './Navigation';

export function AppShell() {
  const [drawerOpen, setDrawerOpen] = useState(false);

  return (
    <div className="min-h-screen bg-canvas">
      <div className="flex min-h-screen">
        <Sidebar />
        <MobileDrawer open={drawerOpen} onClose={() => setDrawerOpen(false)} />
        <div className="flex min-w-0 flex-1 flex-col">
          <Topbar onMenu={() => setDrawerOpen(true)} />
          <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-8">
            <Outlet />
          </main>
        </div>
      </div>
    </div>
  );
}