const API_URL = import.meta.env.VITE_API_URL ?? 'http://localhost:8080';

export class ApiError extends Error {
  constructor(message: string, readonly code: string, readonly status: number) {
    super(message);
    this.name = 'ApiError';
  }
}

interface Envelope {
  error?: { code?: string; message?: string };
  message?: string;
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_URL}${path}`, { credentials: 'include', ...init });
  if (response.status === 204) return undefined as T;
  const body = (await response.json().catch(() => ({}))) as Envelope & T;
  if (!response.ok) {
    throw new ApiError(
      body.error?.message ?? body.message ?? `Request failed (${response.status})`,
      body.error?.code ?? 'request_failed',
      response.status,
    );
  }
  return body;
}

export function post<T>(path: string, data: unknown): Promise<T> {
  return request<T>(path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(data),
  });
}

export function put<T>(path: string, data: unknown): Promise<T> {
  return request<T>(path, {
    method: 'PUT',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(data),
  });
}

export const get = <T>(path: string): Promise<T> => request<T>(path);

export function patch<T>(path: string, data: unknown): Promise<T> {
  return request<T>(path, {
    method: 'PATCH',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(data),
  });
}