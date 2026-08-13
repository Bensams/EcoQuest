import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { cn } from '../../lib/utils';

type Variant = 'primary' | 'secondary' | 'ghost' | 'destructive';
type Size = 'sm' | 'md';

const variants: Record<Variant, string> = {
  primary: 'bg-leaf text-white hover:bg-leaf-dark',
  secondary: 'bg-white border border-sage text-forest hover:bg-sage-soft',
  ghost: 'text-forest-muted hover:bg-sage-soft hover:text-forest',
  destructive: 'bg-red-50 text-red-700 border border-red-200 hover:bg-red-100',
};

const sizes: Record<Size, string> = {
  sm: 'px-3 py-1.5 text-sm',
  md: 'px-4 py-2 text-sm',
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

export function Button({ variant = 'primary', size = 'md', className, ...props }: ButtonProps) {
  return (
    <button
      data-component="button"
      className={cn(
        'inline-flex items-center justify-center gap-2 rounded-lg font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50',
        variants[variant],
        sizes[size],
        className,
      )}
      {...props}
    />
  );
}

interface ButtonLinkProps {
  to: string;
  variant?: Variant;
  size?: Size;
  className?: string;
  children: ReactNode;
}

export function ButtonLink({ to, variant = 'primary', size = 'md', className, children }: ButtonLinkProps) {
  return (
    <Link
      to={to}
      data-component="button"
      className={cn(
        'inline-flex items-center justify-center gap-2 rounded-lg font-medium transition-colors',
        variants[variant],
        sizes[size],
        className,
      )}
    >
      {children}
    </Link>
  );
}