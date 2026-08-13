import type { ReactNode } from 'react';
import { cn } from '../../lib/utils';

export function PageHeader({
  eyebrow,
  title,
  detail,
  action,
  className,
}: {
  eyebrow?: string;
  title: string;
  detail?: string;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div data-component="page-header" className={cn('mb-6 flex items-end justify-between gap-4', className)}>
      <div>
        {eyebrow && <p className="text-xs font-medium uppercase tracking-wider text-forest-muted">{eyebrow}</p>}
        <h1 className="mt-1 text-2xl font-semibold text-forest">{title}</h1>
        {detail && <p className="mt-1 text-sm text-forest-muted">{detail}</p>}
      </div>
      {action}
    </div>
  );
}