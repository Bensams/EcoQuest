import { BadgeCheck, ShieldAlert } from 'lucide-react';
import { Link, useParams } from 'react-router-dom';
import { Badge } from '../../components/ui/Badge';
import { Card } from '../../components/ui/Card';
import { LoadingState } from '../../components/ui/EmptyState';
import { formatDateTime } from '../../lib/format';
import type { Certificate } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

/**
 * Public certificate verification. Deliberately outside the authenticated shell:
 * the point is that an employer or school can check a certificate without an
 * EcoQuest account, using only the hash printed on it.
 *
 * The API returns just the facts already on the certificate — no email, no
 * account identifiers beyond what the holder chose to share.
 */
export function VerifyCertificatePage() {
  const { hash } = useParams<{ hash: string }>();
  const { data: certificate, error, loading } = useFetch<Certificate>(
    hash ? `/api/certificates/public/${encodeURIComponent(hash)}` : null,
    hash,
  );

  const revoked = certificate?.status === 'REVOKED';

  return (
    <main className="min-h-screen bg-surface px-4 py-12">
      <div className="mx-auto max-w-2xl">
        <header className="mb-8 text-center">
          <p className="text-xs font-semibold uppercase tracking-wide text-forest-muted">EcoQuest</p>
          <h1 className="text-2xl font-bold text-forest">Certificate verification</h1>
        </header>

        {loading && <LoadingState label="Checking this certificate…" />}

        {!loading && error && (
          <Card className="text-center">
            <span className="mx-auto mb-3 grid size-12 place-items-center rounded-full bg-red-50 text-red-600">
              <ShieldAlert className="size-6" aria-hidden="true" />
            </span>
            <h2 className="mb-1 text-lg font-semibold text-forest">This certificate could not be verified</h2>
            <p className="mx-auto max-w-md text-sm text-forest-muted">{error}</p>
            <p className="mx-auto mt-3 max-w-md text-sm text-forest-muted">
              Check that the whole verification code was copied. A genuine code is 64 characters long.
            </p>
          </Card>
        )}

        {!loading && !error && certificate && (
          <Card>
            <div className="mb-6 flex flex-col items-center text-center">
              <span
                className={`mb-3 grid size-12 place-items-center rounded-full ${
                  revoked ? 'bg-red-50 text-red-600' : 'bg-green-50 text-green-700'
                }`}
              >
                {revoked ? (
                  <ShieldAlert className="size-6" aria-hidden="true" />
                ) : (
                  <BadgeCheck className="size-6" aria-hidden="true" />
                )}
              </span>
              <h2 className="text-lg font-semibold text-forest">
                {revoked ? 'This certificate has been revoked' : 'This certificate is genuine'}
              </h2>
              <p className="mt-1 text-sm text-forest-muted">
                {revoked
                  ? 'It was issued by EcoQuest but has since been withdrawn.'
                  : 'Issued by EcoQuest for verified volunteer participation.'}
              </p>
              <Badge tone={revoked ? 'destructive' : 'success'} className="mt-3">
                {certificate.status}
              </Badge>
            </div>

            <dl className="grid gap-x-6 gap-y-4 border-t border-sage pt-6 sm:grid-cols-2">
              <Detail label="Certificate number" value={certificate.certificate_number} />
              <Detail label="Volunteer" value={certificate.participant_name} />
              <Detail label="Mission" value={certificate.event_name} />
              <Detail label="Hosted by" value={certificate.organization_name} />
              <Detail label="Issued" value={formatDateTime(certificate.issued_at)} />
              <Detail
                label="Volunteer time"
                value={formatDuration(certificate.volunteer_duration_minutes)}
              />
              <div className="sm:col-span-2">
                <dt className="text-xs font-medium uppercase tracking-wide text-forest-muted">
                  Verification code
                </dt>
                <dd className="mt-1 break-all font-mono text-xs text-forest">
                  {certificate.verification_hash}
                </dd>
              </div>
            </dl>
          </Card>
        )}

        <p className="mt-8 text-center text-sm text-forest-muted">
          <Link to="/app/missions" className="font-medium text-forest underline">
            Explore EcoQuest missions
          </Link>
        </p>
      </div>
    </main>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt className="text-xs font-medium uppercase tracking-wide text-forest-muted">{label}</dt>
      <dd className="mt-1 text-sm font-medium text-forest">{value}</dd>
    </div>
  );
}

function formatDuration(minutes: number): string {
  if (minutes <= 0) return '—';
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (!hours) return `${rest} min`;
  return rest ? `${hours} h ${rest} min` : `${hours} h`;
}
