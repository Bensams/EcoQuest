import { AlertCircle } from 'lucide-react';

export function EmptyState({ title, detail }: { title: string; detail?: string }) {
  return (
    <div className="flex flex-col items-center gap-1.5 rounded-xl border border-dashed border-sage bg-white p-8 text-center">
      <p className="text-sm font-medium text-forest">{title}</p>
      {detail && <p className="text-sm text-forest-muted">{detail}</p>}
    </div>
  );
}

export function ErrorNote({ message }: { message: string }) {
  return (
    <div className="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
      <AlertCircle className="mt-0.5 size-4 shrink-0" />
      <span>{message}</span>
    </div>
  );
}

export function PageHeader({ eyebrow, title, action }: { eyebrow?: string; title: string; action?: React.ReactNode }) {
  return (
    <div className="mb-6 flex items-end justify-between gap-4">
      <div>
        {eyebrow && <p className="text-xs font-medium uppercase tracking-wider text-forest-muted">{eyebrow}</p>}
        <h1 className="mt-1 text-2xl font-semibold text-forest">{title}</h1>
      </div>
      {action}
    </div>
  );
}