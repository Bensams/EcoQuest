import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './index.css';
import { AuthProvider } from './lib/auth';
import { AppShell } from './components/layout/AppShell';
import { ProtectedRoute } from './router/guards';
import { LoginPage, RegisterPage } from './pages/auth/AuthPages';
import { DashboardPage } from './pages/player/DashboardPage';
import { MissionsPage } from './pages/player/MissionsPage';
import { MissionDetailPage } from './pages/player/MissionDetailPage';
import { MyActivitiesPage } from './pages/player/MyActivitiesPage';
import { AchievementsPage } from './pages/player/AchievementsPage';
import { CertificatesPage } from './pages/player/CertificatesPage';
import { ImpactPage } from './pages/player/ImpactPage';
import { ProfilePage } from './pages/player/ProfilePage';
import { CheckInPage } from './pages/player/CheckInPage';
import { WalletPage } from './pages/player/WalletPage';
import { OrgHomePage } from './pages/org/OrgHomePage';
import { CreateOrganizationPage } from './pages/org/CreateOrganizationPage';
import { OrgEventsPage } from './pages/org/OrgEventsPage';
import { OrgEventDetailPage } from './pages/org/OrgEventDetailPage';
import { AdminHomePage } from './pages/admin/AdminHomePage';

function AppRoutes() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route path="/register" element={<RegisterPage />} />

      <Route
        element={
          <ProtectedRoute>
            <AppShell />
          </ProtectedRoute>
        }
      >
        <Route index element={<Navigate to="/app/dashboard" replace />} />
        <Route path="/app/dashboard" element={<DashboardPage />} />
        <Route path="/app/missions" element={<MissionsPage />} />
        <Route path="/app/missions/:id" element={<MissionDetailPage />} />
        <Route path="/app/activities" element={<MyActivitiesPage />} />
        <Route path="/app/achievements" element={<AchievementsPage />} />
        <Route path="/app/certificates" element={<CertificatesPage />} />
        <Route path="/app/impact" element={<ImpactPage />} />
        <Route path="/app/profile" element={<ProfilePage />} />
        <Route path="/app/check-in" element={<CheckInPage />} />
        <Route path="/app/wallet" element={<WalletPage />} />
        <Route
          path="/org"
          element={<ProtectedRoute allow={['USER', 'ADMIN']}><OrgHomePage /></ProtectedRoute>}
        />
        <Route
          path="/org/create"
          element={<ProtectedRoute allow={['USER', 'ADMIN']}><CreateOrganizationPage /></ProtectedRoute>}
        />
        <Route
          path="/org/:organizationId/events"
          element={<ProtectedRoute allow={['USER', 'ADMIN']}><OrgEventsPage /></ProtectedRoute>}
        />
        <Route
          path="/org/:organizationId/events/:eventId"
          element={<ProtectedRoute allow={['USER', 'ADMIN']}><OrgEventDetailPage /></ProtectedRoute>}
        />
        <Route
          path="/admin"
          element={<ProtectedRoute allow={['ADMIN']}><AdminHomePage /></ProtectedRoute>}
        />
      </Route>

      <Route path="*" element={<Navigate to="/app/missions" replace />} />
    </Routes>
  );
}

function Root() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <AppRoutes />
      </AuthProvider>
    </BrowserRouter>
  );
}

const container = document.getElementById('root');
if (!container) throw new Error('root element not found');

createRoot(container).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);