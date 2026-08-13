import { useState, type ChangeEvent, type FormEvent } from 'react';
import { QrCode } from 'lucide-react';
import { Card } from '../../components/ui/Card';
import { Button } from '../../components/ui/Button';
import { EmptyState, ErrorNote } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { Field, TextInput } from '../../components/ui/Field';
import { post } from '../../lib/api';
import { useAuth } from '../../lib/auth';
import type { Participation } from '../../lib/types';

type BarcodeDetectorLike = { detect(source: ImageBitmapSource): Promise<{ rawValue: string }[]> };

declare global {
  interface Window { BarcodeDetector?: new () => BarcodeDetectorLike }
}

export function CheckInPage() {
  const { user } = useAuth();
  const [code, setCode] = useState('');
  const [pending, setPending] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const checkIn = async (value: string) => {
    if (!value.trim()) {
      setError('Enter a check-in code.');
      setResult(null);
      return;
    }
    setError(null);
    setResult(null);
    setPending(true);
    try {
      const participation = await post<Participation>('/api/check-in', { code: value.trim() });
      setResult(
        participation.status === 'VERIFIED'
          ? 'Checked in and verified.'
          : 'Checked in. The organizer will verify your participation.',
      );
      setCode('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Check-in failed.');
    } finally {
      setPending(false);
    }
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    void checkIn(code);
  };

  const scanImage = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;
    if (!window.BarcodeDetector) {
      setError('Camera scanning is unavailable here — type the code from the QR instead.');
      return;
    }
    try {
      const values = await new window.BarcodeDetector().detect(await createImageBitmap(file));
      if (!values[0]) throw new Error('No QR code found in that image.');
      setCode(values[0].rawValue);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Could not read the QR image.');
    }
  };

  if (!user) return <EmptyState title="Sign in to check in" />;

  return (
    <div>
      <PageHeader eyebrow="Check-in" title="Check in to a mission" />
      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <h2 className="mb-4 flex items-center gap-2 text-sm font-semibold text-forest">
            <QrCode className="size-4" aria-hidden="true" /> Scan or enter a code
          </h2>
          <form onSubmit={submit} className="grid gap-3">
            <Field label="Check-in code">
              <TextInput value={code} onChange={(e) => setCode(e.target.value)} autoComplete="off" placeholder="e.g. XK4T-9MPQ" />
            </Field>
            {error && <ErrorNote message={error} />}
            {result && <p className="rounded-lg bg-sage-soft px-3 py-2 text-sm text-forest">{result}</p>}
            <Button type="submit" disabled={pending}>
              {pending ? 'Checking in…' : 'Check in'}
            </Button>
          </form>
        </Card>
        <Card>
          <h2 className="mb-2 text-sm font-semibold text-forest">Scan a QR with your camera</h2>
          <p className="mb-4 text-sm text-forest-muted">Capture the organizer's QR code or upload a screenshot.</p>
          <label className="inline-flex cursor-pointer items-center justify-center rounded-lg border border-sage bg-white px-4 py-2 text-sm font-medium text-forest transition-colors hover:bg-sage-soft">
            Choose image or scan
            <input type="file" accept="image/*" capture="environment" className="sr-only" onChange={(e) => void scanImage(e)} />
          </label>
        </Card>
      </div>
    </div>
  );
}