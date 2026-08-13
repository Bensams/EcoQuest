import type { BadgeTone } from '../components/ui/Badge';
import type { EventStatus, ParticipationStatus, VerificationStatus } from './types';

const activityLabels: Record<string, string> = {
  TREE_PLANTING: 'Tree planting',
  WASTE_COLLECTION: 'Waste collection',
  RECYCLING: 'Recycling',
  BEACH_CLEANUP: 'Beach cleanup',
  ENERGY_SAVING: 'Energy saving',
  EDUCATION: 'Education',
  OTHER: 'Other',
};

const eventStatusToTone: Record<EventStatus, BadgeTone> = {
  DRAFT: 'muted',
  PUBLISHED: 'info',
  ACTIVE: 'success',
  COMPLETED: 'success',
  CANCELLED: 'destructive',
};

const participationStatusToTone: Record<ParticipationStatus, BadgeTone> = {
  REGISTERED: 'warning',
  PENDING_VERIFICATION: 'warning',
  VERIFIED: 'success',
  REJECTED: 'destructive',
  CANCELLED: 'destructive',
};

const verificationToTone: Record<VerificationStatus, BadgeTone> = {
  PENDING: 'warning',
  APPROVED: 'success',
  REJECTED: 'destructive',
  SUSPENDED: 'destructive',
};

const statusLabels: Record<string, string> = {
  DRAFT: 'Draft',
  PUBLISHED: 'Published',
  ACTIVE: 'Active',
  COMPLETED: 'Completed',
  CANCELLED: 'Cancelled',
  REGISTERED: 'Registered',
  PENDING_VERIFICATION: 'Pending verification',
  VERIFIED: 'Verified',
  REJECTED: 'Rejected',
  PENDING: 'Pending review',
  APPROVED: 'Approved',
  SUSPENDED: 'Suspended',
};

export function activityLabel(type: string): string {
  return activityLabels[type] ?? 'Other';
}

export function eventStatusTone(status: EventStatus): BadgeTone {
  return eventStatusToTone[status];
}

export function statusToneOf(status: EventStatus): BadgeTone {
  return eventStatusToTone[status];
}

export function participationStatusTone(status: ParticipationStatus): BadgeTone {
  return participationStatusToTone[status];
}

export function verificationToneOf(status: VerificationStatus): BadgeTone {
  return verificationToTone[status];
}

export function statusLabel(status: string): string {
  return statusLabels[status] ?? status;
}

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
}

export function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });
}

export function formatDateRange(startsAt: string, endsAt: string): string {
  const start = new Date(startsAt);
  const end = new Date(endsAt);
  const date = start.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  const startTime = start.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
  const endTime = end.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
  return `${date} · ${startTime} – ${endTime}`;
}