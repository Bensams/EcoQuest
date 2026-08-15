import { useState } from 'react';
import { Button } from '../ui/Button';
import { Dialog } from '../ui/Dialog';
import { Field, TextArea } from '../ui/Field';

export function ReasonDialog({
  open,
  title,
  description,
  confirmLabel,
  destructive = false,
  busy = false,
  onClose,
  onConfirm,
}: {
  open: boolean;
  title: string;
  description?: string;
  confirmLabel: string;
  destructive?: boolean;
  busy?: boolean;
  onClose: () => void;
  onConfirm: (reason: string) => Promise<void> | void;
}) {
  const [reason, setReason] = useState('');

  const close = () => {
    if (busy) return;
    setReason('');
    onClose();
  };

  return (
    <Dialog open={open} title={title} description={description} onClose={close}>
      <form
        className="grid gap-4"
        onSubmit={(event) => {
          event.preventDefault();
          if (!reason.trim()) return;
          void onConfirm(reason.trim());
        }}
      >
        <Field label="Reason">
          <TextArea
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            placeholder="Required. This is stored on the audit trail."
            required
          />
        </Field>
        <div className="flex justify-end gap-2">
          <Button type="button" variant="secondary" onClick={close} disabled={busy}>
            Cancel
          </Button>
          <Button type="submit" variant={destructive ? 'destructive' : 'primary'} disabled={busy || !reason.trim()}>
            {busy ? 'Saving…' : confirmLabel}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
