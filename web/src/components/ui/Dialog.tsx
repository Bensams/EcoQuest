import { Dialog as AriaDialog, Modal, ModalOverlay } from 'react-aria-components';
import { X } from 'lucide-react';
import type { ReactNode } from 'react';
import { cn } from '../../lib/utils';

export function Dialog({
  open,
  title,
  description,
  children,
  onClose,
  className,
}: {
  open: boolean;
  title: string;
  description?: string;
  children: ReactNode;
  onClose: () => void;
  className?: string;
}) {
  return (
    <ModalOverlay
      isOpen={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose();
      }}
      className="fixed inset-0 z-50 grid place-items-center overflow-y-auto bg-forest/40 p-4"
    >
      <Modal
        data-component="dialog"
        className={cn('w-full max-w-md rounded-xl bg-surface p-6 shadow-xl outline-none', className)}
      >
        <AriaDialog className="outline-none">
          <div className="mb-4 flex items-start justify-between gap-4">
            <div>
              <h2 slot="title" className="text-lg font-semibold text-forest">{title}</h2>
              {description && <p className="mt-1 text-sm text-forest-muted">{description}</p>}
            </div>
            <button
              className="rounded-lg p-1.5 text-forest-muted transition-colors hover:bg-sage-soft hover:text-forest"
              onClick={onClose}
              aria-label="Close dialog"
            >
              <X className="size-5" aria-hidden="true" />
            </button>
          </div>
          {children}
        </AriaDialog>
      </Modal>
    </ModalOverlay>
  );
}
