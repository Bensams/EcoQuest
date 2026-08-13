import type {
  InputHTMLAttributes,
  LabelHTMLAttributes,
  ReactNode,
  TextareaHTMLAttributes,
} from 'react';
import {
  Button as AriaButton,
  ListBox,
  ListBoxItem,
  Popover,
  Select as BaseSelect,
  SelectValue,
} from 'react-aria-components';
import { ChevronDown } from 'lucide-react';
import { cn } from '../../lib/utils';

export function Field({
  label,
  children,
  hint,
  error,
  ...rest
}: { label: string; children: ReactNode; hint?: string; error?: string } & LabelHTMLAttributes<HTMLLabelElement>) {
  return (
    <label className="grid gap-1.5 text-sm font-medium text-forest" {...rest}>
      <span>{label}</span>
      {children}
      {hint && !error && <span className="text-xs font-normal text-forest-muted">{hint}</span>}
      {error && <span className="text-xs font-normal text-red-600">{error}</span>}
    </label>
  );
}

const inputClasses = cn(
  'rounded-lg border border-sage bg-surface px-3 py-2 text-sm text-forest placeholder:text-forest-muted/60 outline-none transition',
  'focus:border-leaf focus:ring-2 focus:ring-leaf/20',
  'aria-invalid:border-red-300 aria-invalid:focus:ring-red-200',
);

export function TextInput(props: InputHTMLAttributes<HTMLInputElement>) {
  return <input data-component="text-input" className={inputClasses} {...props} />;
}

export function TextArea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea data-component="textarea" className={cn(inputClasses, 'min-h-24')} {...props} />;
}

export function Select({
  value,
  onChange,
  options,
  placeholder,
  className,
  'aria-label': ariaLabel,
}: {
  value: string;
  onChange: (value: string) => void;
  options: Array<{ value: string; label: string }>;
  placeholder?: string;
  className?: string;
  'aria-label'?: string;
}) {
  return (
    <BaseSelect
      data-component="select"
      selectedKey={value || undefined}
      onSelectionChange={(key) => onChange(String(key === null ? '' : key))}
      className={cn('block w-full', className)}
      aria-label={ariaLabel}
    >
      <AriaButton className={cn(inputClasses, 'flex w-full items-center justify-between gap-2')}>
        <SelectValue className="truncate">
          {({ selectedText }) => (
            <span className={selectedText ? '' : 'text-forest-muted/60'}>{selectedText || placeholder}</span>
          )}
        </SelectValue>
        <ChevronDown className="size-4 shrink-0 text-forest-muted" aria-hidden="true" />
      </AriaButton>
      <Popover
        className={cn(
          'rounded-lg border border-sage bg-surface shadow-lg outline-none',
          'min-w-[var(--trigger-width)]',
        )}
      >
        <ListBox className="max-h-60 overflow-auto p-1 outline-none">
          {options.map((option) => (
            <ListBoxItem
              key={option.value}
              id={option.value}
              className={({ isFocused, isSelected }) =>
                cn(
                  'cursor-pointer rounded-md px-3 py-2 text-sm text-forest outline-none',
                  isSelected && 'bg-sage-soft font-medium text-leaf',
                  isFocused && 'bg-sage-soft',
                )
              }
            >
              {option.label}
            </ListBoxItem>
          ))}
        </ListBox>
      </Popover>
    </BaseSelect>
  );
}