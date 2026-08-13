import type { InputHTMLAttributes, LabelHTMLAttributes, ReactNode } from 'react';

export function Field({ label, children, ...rest }: { label: string; children: ReactNode } & LabelHTMLAttributes<HTMLLabelElement>) {
  return (
    <label className="grid gap-1.5 text-sm font-medium text-forest" {...rest}>
      <span>{label}</span>
      {children}
    </label>
  );
}

export function TextInput(props: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className="rounded-lg border border-sage bg-white px-3 py-2 text-sm text-forest placeholder:text-forest-muted/60 outline-none transition focus:border-leaf focus:ring-2 focus:ring-leaf/20"
      {...props}
    />
  );
}