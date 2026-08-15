import { Award, ExternalLink } from 'lucide-react';
import { Badge } from '../../components/ui/Badge';
import { Card, StatTile } from '../../components/ui/Card';
import { EmptyState } from '../../components/ui/EmptyState';
import { PageHeader } from '../../components/ui/PageHeader';
import { useAuth } from '../../lib/auth';
import { formatDateTime } from '../../lib/format';
import type { Certificate } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

function statusTone(status: string) {
  if (status.includes('MINTED') || status.includes('ISSUED') || status === 'ISSUED' || status === 'VALID') return 'success';
  if (status.includes('REVOKED') || status.includes('FAILED')) return 'destructive';
  return 'muted';
}

function verifyUrl(hash: string): string {
  const base = import.meta.env.VITE_APP_ORIGIN ?? window.location.origin;
  return `${base}/certificates/verify/${encodeURIComponent(hash)}`;
}

export function CertificatesPage() {
  const { user } = useAuth();
  const { data: certificates, loading, error } = useFetch<Certificate[]>(
    user ? '/api/certificates/me' : null,
    user?.id,
  );

  if (!user) return <EmptyState title="Sign in to view your certificates" detail="Issued certificates appear here once your participation is verified." />;

  const issued = certificates?.length ?? 0;

  return (
    <div>
      <PageHeader
        eyebrow="Certificates"
        title="Your certificates"
        detail="Downloadable, verifiable records of your completed volunteer work."
      />
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}

      <section className="mb-8 grid grid-cols-2 gap-4 md:grid-cols-3">
        <StatTile label="Issued" value={String(issued)} />
      </section>

      {!loading && !error && (!certificates || certificates.length === 0) && (
        <EmptyState title="No certificates yet" detail="Certificates are issued automatically after your organizer verifies a completed participation." />
      )}

      {loading && <p className="text-sm text-forest-muted">Loading certificates…</p>}

      {!loading && !error && certificates && (
        <ul className="space-y-4">
          {certificates.map((c) => (
            <Card key={c.certificate_number} className="flex flex-wrap gap-4 items-center justify-between">
              <div className="flex items-center gap-3">
                <span className="grid size-10 shrink-0 place-items-center rounded-lg bg-sage-soft text-forest">
                  <Award className="size-5" aria-hidden="true" />
                </span>
                <div>
                  <p className="font-semibold text-forest">{c.certificate_number}</p>
                  <p className="text-sm text-forest-muted">{c.event_name} · {c.organization_name}</p>
                </div>
              </div>
              <div className="flex flex-wrap items-center gap-2 text-sm">
                <span className="text-xs text-forest-muted">
                  Issued {formatDateTime(c.issued_at)}
                </span>
                <Badge tone={statusTone(c.status)}>{c.status}</Badge>
                <a
                  href={verifyUrl(c.verification_hash)}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1 rounded-lg border border-sage bg-white px-2.5 py-1 text-xs font-medium text-forest transition-colors hover:bg-sage-soft"
                  aria-label={`Verify certificate ${c.certificate_number}`}
                >
                  <ExternalLink className="size-3.5" aria-hidden="true" /> Verify
                </a>
              </div>
            </Card>
          ))}
        </ul>
      )}
    </div>
  );
}
