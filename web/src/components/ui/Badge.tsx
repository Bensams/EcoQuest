import type { ReactNode } from 'react';
import { cn } from '../../lib/utils';

export type BadgeTone = 'success' | 'warning' | 'destructive' | 'muted' | 'info';

const tones: Record<BadgeTone, string> = {
  success: 'bg-green-50 text-green-700',
  warning: 'bg-amber-soft text-amber',
  muted: 'bg-sage-soft text-forest-muted',
  destructive: 'bg-red-50 text-red-700',
  info: 'bg-blue-50 text-blue-700',
};

const dotTones: Record<BadgeTone, string> = {
  success: 'bg-green-500',
  warning: 'bg-amber-500',
  muted: 'bg-gray-400',
  destructive: 'bg-red-500',
  info: 'bg-blue-500',
};

/**
 * Semantic status badge. Map domain statuses to one of the semantic tones in
 * `lib/format`; the palette lives in the central theme.
 */
export function Badge({ tone = 'muted', className, children }: { tone?: BadgeTone; className?: string; children: ReactNode }) {
  return (
    <span
      data-component="badge"
      className={cn('inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium', tones[tone], className)}
    >
      <span className={cn('size-1.5 rounded-full', dotTones[tone])} aria-hidden="true" />
      {children}
    </span>
  );
}