export type AdminQuery = {
  q: string;
  status: string;
  organization: string;
  activity: string;
  location: string;
  from: string;
  to: string;
  flagged: boolean;
  sort: string;
  order: 'asc' | 'desc';
  page: number;
  perPage: number;
};

export const defaultAdminQuery = (sort = 'created_at'): AdminQuery => ({
  q: '',
  status: '',
  organization: '',
  activity: '',
  location: '',
  from: '',
  to: '',
  flagged: false,
  sort,
  order: 'desc',
  page: 1,
  perPage: 20,
});

export function adminListPath(base: string, query: AdminQuery): string {
  const params = new URLSearchParams();
  if (query.q.trim()) params.set('q', query.q.trim());
  if (query.status) params.set('status', query.status);
  if (query.organization) params.set('organization_id', query.organization);
  if (query.activity) params.set('activity_type', query.activity);
  if (query.location.trim()) params.set('location', query.location.trim());
  if (query.from) params.set('from', new Date(query.from).toISOString());
  if (query.to) params.set('to', new Date(query.to).toISOString());
  if (query.flagged) params.set('flagged', 'true');
  if (query.sort) params.set('sort', query.sort);
  params.set('order', query.order);
  params.set('page', String(query.page));
  params.set('per_page', String(query.perPage));
  return `${base}?${params.toString()}`;
}

export function pageCount(total: number, perPage: number): number {
  return Math.max(1, Math.ceil(total / perPage));
}
