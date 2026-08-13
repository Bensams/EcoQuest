import type { EventStatus } from '../lib/types';

const activityLabels: Record<string, string> = {
  TREE_PLANTING: 'Tree planting',
  WASTE_COLLECTION: 'Waste collection',
  RECYCLING: 'Recycling',
  BEACH_CLEANUP: 'Beach cleanup',
  ENERGY_SAVING: 'Energy saving',
  EDUCATION: 'Education',
  OTHER: 'Other',
};

const statusTone: Record<EventStatus, 'green' | 'amber' | 'gray' | 'blue' | 'red'> = {
  DRAFT: 'gray',
  PUBLISHED: 'blue',
  ACTIVE: 'green',
  COMPLETED: 'green',
  CANCELLED: 'red',
};

export function activityLabel(type: string): string {
  return activityLabels[type] ?? 'Other';
}

export function statusToneOf(status: EventStatus) {
  return statusTone[status];
}

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
}

export function formatDateRange(startsAt: string, endsAt: string): string {
  const start = new Date(startsAt);
  const end = new Date(endsAt);
  const date = start.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  const startTime = start.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
  const endTime = end.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
  return `${date} · ${startTime} – ${endTime}`;
}