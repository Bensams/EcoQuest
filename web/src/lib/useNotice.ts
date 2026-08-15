import { useCallback, useState } from 'react';
import type { NoticeTone } from '../components/ui/EmptyState';

export type NoticeState = { tone: NoticeTone; message: string } | null;

/**
 * One-off result banner for pages whose actions can succeed or fail.
 *
 * The tone travels with the message so a confirmation can never be rendered in
 * the error palette — the bug this replaces was every page storing a bare
 * string and always painting it red.
 */
export function useNotice() {
  const [notice, setNotice] = useState<NoticeState>(null);
  const clear = useCallback(() => setNotice(null), []);
  const succeed = useCallback((message: string) => setNotice({ tone: 'success', message }), []);
  /** Prefers the API's own message, which carries the actionable reason. */
  const fail = useCallback(
    (error: unknown, fallback: string) =>
      setNotice({ tone: 'error', message: error instanceof Error ? error.message : fallback }),
    [],
  );
  return { notice, clear, succeed, fail };
}
