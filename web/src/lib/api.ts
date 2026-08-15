// Empty means same-origin: the dev server proxies /api to the API, and a
// deployment serves both behind one origin. Set VITE_API_URL only when the API
// really is on another origin, and expect to configure CORS if you do.
const API_URL = import.meta.env.VITE_API_URL ?? '';

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

/// Paths that must never trigger a token refresh: refreshing on their 401 would
/// either recurse or mask a genuine bad-credentials answer.
const NO_REFRESH = ['/api/auth/login', '/api/auth/register', '/api/auth/refresh', '/api/auth/logout'];

/// Notified when the session is definitively gone, so the UI can stop claiming
/// the user is signed in.
type SessionLostHandler = () => void;
let onSessionLost: SessionLostHandler | null = null;

export function setSessionLostHandler(handler: SessionLostHandler | null) {
  onSessionLost = handler;
}

/// Shared across concurrent callers: a page that fires several requests at once
/// must rotate the refresh token once, not once per request. Rotation is
/// single-use server-side, so parallel refreshes would revoke the token family.
let inFlightRefresh: Promise<boolean> | null = null;

async function refreshSession(): Promise<boolean> {
  inFlightRefresh ??= (async () => {
    try {
      const response = await fetch(`${API_URL}/api/auth/refresh`, {
        method: 'POST',
        credentials: 'include',
      });
      return response.ok;
    } catch {
      return false;
    } finally {
      // Cleared in a microtask so callers awaiting this attempt share its result.
      queueMicrotask(() => {
        inFlightRefresh = null;
      });
    }
  })();
  return inFlightRefresh;
}

async function send(path: string, init?: RequestInit): Promise<Response> {
  return fetch(`${API_URL}${path}`, { credentials: 'include', ...init });
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let response = await send(path, init);

  // The access token lives ~15 minutes. Rather than let the app rot into a
  // wall of "authentication required", rotate the refresh token once and retry.
  if (response.status === 401 && !NO_REFRESH.includes(path)) {
    if (await refreshSession()) {
      response = await send(path, init);
    } else {
      onSessionLost?.();
    }
  }

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