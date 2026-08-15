import { AlertCircle, CheckCircle2, Info, Loader2, SearchX } from 'lucide-react';
import type { ComponentType, ReactNode } from 'react';
import { cn } from '../../lib/utils';
import { Button } from './Button';

export function EmptyState({
  title,
  detail,
  action,
  className,
}: {
  title: string;
  detail?: string;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div
      data-component="empty-state"
      className={cn('flex flex-col items-center gap-2 rounded-xl border border-dashed border-sage bg-surface p-10 text-center', className)}
    >
      <span className="grid size-10 place-items-center rounded-full bg-sage-soft text-forest-muted">
        <SearchX className="size-5" aria-hidden="true" />
      </span>
      <p className="text-sm font-medium text-forest">{title}</p>
      {detail && <p className="max-w-sm text-sm text-forest-muted">{detail}</p>}
      {action}
    </div>
  );
}

export type NoticeTone = 'success' | 'error' | 'info';

const noticeStyles: Record<NoticeTone, { className: string; Icon: ComponentType<{ className?: string }> }> = {
  success: { className: 'border-green-200 bg-green-50 text-green-800', Icon: CheckCircle2 },
  error: { className: 'border-red-200 bg-red-50 text-red-700', Icon: AlertCircle },
  info: { className: 'border-sage bg-sage-soft text-forest', Icon: Info },
};

/**
 * Inline result banner. Actions that can succeed *or* fail must pass the tone
 * that matches the outcome — a green confirmation reads as a failure when it is
 * rendered in the error palette.
 *
 * `role` follows the tone: only errors interrupt a screen reader; successes are
 * announced politely.
 */
export function Notice({ tone = 'error', message }: { tone?: NoticeTone; message: string }) {
  const { className, Icon } = noticeStyles[tone];
  return (
    <div
      data-component="notice"
      data-tone={tone}
      className={cn('flex items-start gap-2 rounded-lg border px-3 py-2 text-sm', className)}
      role={tone === 'error' ? 'alert' : 'status'}
    >
      <Icon className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
      <span>{message}</span>
    </div>
  );
}

/** [`Notice`] fixed to the error tone, for call sites that only ever fail. */
export function ErrorNote({ message }: { message: string }) {
  return <Notice tone="error" message={message} />;
}

export function ErrorState({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <EmptyState
      title="Something went wrong"
      detail={message}
      action={onRetry && <Button variant="secondary" size="sm" onClick={onRetry}>Try again</Button>}
    />
  );
}

export function LoadingState({ label = 'Loading…' }: { label?: string }) {
  return (
    <div data-component="loading-state" className="flex items-center gap-2 py-8 text-sm text-forest-muted" role="status">
      <Loader2 className="size-4 animate-spin" aria-hidden="true" />
      {label}
    </div>
  );
}

export function Skeleton({ className }: { className?: string }) {
  return <div data-component="skeleton" className={cn('animate-pulse rounded-lg bg-sage-soft', className)} aria-hidden="true" />;
}