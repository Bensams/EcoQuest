import { useCallback, useState } from 'react';
import { isConnected, requestAccess, signMessage } from '@stellar/freighter-api';
import { Card } from '../../components/ui/Card';
import { Button } from '../../components/ui/Button';
import { EmptyState, PageHeader } from '../../components/ui/EmptyState';
import { post } from '../../lib/api';
import { useAuth } from '../../lib/auth';
import type { Achievement } from '../../lib/types';
import { useFetch } from '../../lib/useFetch';

type SolanaProvider = {
  connect(): Promise<{ publicKey: { toString(): string } }>;
  signMessage(message: Uint8Array, encoding?: string): Promise<{ signature: Uint8Array } | Uint8Array>;
};

declare global {
  interface Window { solana?: SolanaProvider }
}

type WalletChallenge = { message: string; nonce: string; expires_at: string };
type WalletHandle = { address: string; sign(text: string): Promise<string> };

function toBase64(bytes: Uint8Array): string {
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

async function connectFreighter(): Promise<WalletHandle> {
  const connected = await isConnected();
  if (!connected.isConnected) throw new Error('No Freighter wallet found. Install the Freighter extension.');
  const access = await requestAccess();
  if (!access.address) throw new Error(access.error ?? 'Freighter did not return an address.');
  const address = access.address;
  return {
    address,
    sign: async (text: string) => {
      const signed = await signMessage(text, { address });
      if (signed.error) throw new Error(signed.error);
      if (typeof signed.signedMessage === 'string') return signed.signedMessage;
      if (signed.signedMessage) return toBase64(signed.signedMessage as unknown as Uint8Array);
      throw new Error('Freighter did not return a signature.');
    },
  };
}

async function connectSolana(): Promise<WalletHandle> {
  const provider = window.solana;
  if (!provider) throw new Error('No Solana wallet found. Install Phantom, Solflare or Backpack.');
  const { publicKey } = await provider.connect();
  return {
    address: publicKey.toString(),
    sign: async (text: string) => {
      const signed = await provider.signMessage(new TextEncoder().encode(text), 'utf8');
      return toBase64(signed instanceof Uint8Array ? signed : signed.signature);
    },
  };
}

export function WalletPage() {
  const { user, refresh } = useAuth();
  const { data: achievements, reload: reloadAchievements } = useFetch<Achievement[]>(
    user ? '/api/achievements/me' : null,
    user?.wallet_address,
  );
  const [inflight, setInflight] = useState<null | 'Stellar' | 'Solana'>(null);
  const [error, setError] = useState<string | null>(null);

  const linkWallet = useCallback(
    async (connect: () => Promise<WalletHandle>) => {
      if (!user) return;
      setError(null);
      try {
        const wallet = await connect();
        const challenge = await post<WalletChallenge>('/api/wallet/challenge', { wallet_address: wallet.address });
        const signature = await wallet.sign(challenge.message);
        await post<void>('/api/wallet/verify', { wallet_address: wallet.address, nonce: challenge.nonce, signature });
        await refresh();
        await reloadAchievements();
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Could not link wallet.');
      } finally {
        setInflight(null);
      }
    },
    [user, refresh, reloadAchievements],
  );

  if (!user) return <EmptyState title="Sign in to manage your wallet and achievements" />;

  return (
    <div>
      <PageHeader eyebrow="Achievements" title="Wallet & recognition" />
      {error && <p className="mb-4 text-sm text-red-600">{error}</p>}

      <Card className="mb-6">
        <h2 className="mb-1 text-base font-semibold text-forest">Link a wallet</h2>
        <p className="mb-4 text-sm text-forest-muted">
          Link a wallet to prove ownership of your achievements. Signing only proves you control the key — it never moves funds.
        </p>
        {user.wallet_address ? (
          <p className="rounded-lg bg-sage-soft px-3 py-2 text-sm text-forest">
            Linked: <span className="font-mono">{user.wallet_address}</span>
          </p>
        ) : (
          <div className="flex gap-3">
            <Button
              onClick={() => {
                setInflight('Stellar');
                void linkWallet(connectFreighter);
              }}
              disabled={inflight !== null}
            >
              {inflight === 'Stellar' ? 'Waiting for wallet…' : 'Connect Freighter (Stellar)'}
            </Button>
            <Button
              variant="secondary"
              onClick={() => {
                setInflight('Solana');
                void linkWallet(connectSolana);
              }}
              disabled={inflight !== null}
            >
              {inflight === 'Solana' ? 'Waiting for wallet…' : 'Connect Solana wallet'}
            </Button>
          </div>
        )}
      </Card>

      <section className="grid gap-4 md:grid-cols-2">
        <AchievementsPanel achievements={achievements} />
        <CertificatesPanel />
      </section>
    </div>
  );
}
  function AchievementsPanel({ achievements }: { achievements: Achievement[] | null }) {
  if (!achievements || achievements.length === 0) {
    return (
      <Card>
        <h2 className="mb-3 text-sm font-semibold text-forest">Achievements</h2>
        <p className="text-sm text-forest-muted">No achievements yet. Verified participation unlocks them.</p>
      </Card>
    );
  }
  return (
    <Card>
      <h2 className="mb-3 text-sm font-semibold text-forest">Achievements</h2>
      <ul className="space-y-3">
        {achievements.map((item) => (
          <li key={item.achievement_key} className="flex items-center justify-between gap-3">
            <div>
              <p className="text-sm font-medium text-forest">{item.achievement_key.replace(/_/g, ' ')}</p>
              <p className="text-xs text-forest-muted">Ref {item.verification_reference}</p>
            </div>
            <BadgeTone status={item.status} />
          </li>
        ))}
      </ul>
    </Card>
  );
}

function CertificatesPanel() {
  return (
    <Card>
      <h2 className="mb-3 text-sm font-semibold text-forest">Certificates</h2>
      <p className="text-sm text-forest-muted">
        Issued certificates are verified by their public hash. The full certificate history ships with the organizer console.
      </p>
    </Card>
  );
}

function BadgeTone({ status }: { status: string }) {
  const tone = status.includes('ELIGIBLE') || status.includes('MINTED') ? 'bg-green-50 text-green-700' : 'bg-sage-soft text-forest-muted';
  return <span className={`rounded-full px-2.5 py-0.5 text-xs font-medium ${tone}`}>{status}</span>;
}