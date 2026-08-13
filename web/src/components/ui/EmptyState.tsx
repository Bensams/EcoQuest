import { AlertCircle, Loader2, SearchX } from 'lucide-react';
import type { ReactNode } from 'react';
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

export function ErrorNote({ message }: { message: string }) {
  return (
    <div
      data-component="error-note"
      className="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700"
      role="alert"
    >
      <AlertCircle className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
      <span>{message}</span>
    </div>
  );
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