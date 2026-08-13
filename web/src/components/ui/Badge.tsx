import type { ReactNode } from 'react';

type Tone = 'green' | 'amber' | 'gray' | 'red' | 'blue';

const tones: Record<Tone, string> = {
  green: 'bg-green-50 text-green-700',
  amber: 'bg-amber-soft text-amber',
  gray: 'bg-sage-soft text-forest-muted',
  red: 'bg-red-50 text-red-700',
  blue: 'bg-blue-50 text-blue-700',
};

const dotTones: Record<Tone, string> = {
  green: 'bg-green-500',
  amber: 'bg-amber-500',
  gray: 'bg-gray-400',
  red: 'bg-red-500',
  blue: 'bg-blue-500',
};

export function Badge({ tone = 'gray', children }: { tone?: Tone; children: ReactNode }) {
  return (
    <span className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium ${tones[tone]}`}>
      <span className={`size-1.5 rounded-full ${dotTones[tone]}`} aria-hidden="true" />
      {children}
    </span>
  );
}