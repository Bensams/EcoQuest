import { ChangeEvent, useEffect, useRef, useState } from 'react';

const API_URL = import.meta.env.VITE_API_URL ?? 'http://localhost:8080';
// Built by wasm-bindgen into public/, so it is served as a static asset rather than
// bundled. An absolute URL keeps Vite from resolving it through the module graph.
const GAME_WASM_URL = new URL('/game/ecoquest_game_wasm.js', import.meta.url).href;
type Impact = { metric: string; unit: string; expected_value: number };
type Event = { id: string; name: string; description: string; activity_type: string; location: string; starts_at: string; ends_at: string; capacity: number; eco_points: number; registered_count: number; impacts: Impact[] };
type Profile = { username: string; eco_points: number; wallet_address?: string };
type Achievement = { achievement_key: string; status: string; verification_reference: string; mint_identifier?: string; transaction_signature?: string; explorer_url?: string };
type GameState = { level: number; progress_points: number; points_to_next_level: number; character: string; environment: string; restoration_stage: number };
type BarcodeDetectorLike = { detect(source: ImageBitmapSource): Promise<{ rawValue: string }[]> };
// Phantom, Solflare and Backpack all expose this provider shape on window.solana.
type SolanaProvider = { connect(): Promise<{ publicKey: { toString(): string } }>; signMessage(message: Uint8Array, encoding?: string): Promise<{ signature: Uint8Array } | Uint8Array> };
// Freighter (Stellar) injects window.freighterApi. Signatures come back base64 encoded.
type FreighterApi = {
  isConnected(): Promise<boolean | { isConnected: boolean }>;
  setAllowed?(): Promise<unknown>;
  requestAccess?(): Promise<{ address: string; error?: string }>;
  getAddress?(): Promise<{ address: string; error?: string }>;
  getPublicKey?(): Promise<string>;
  signMessage(message: string, options?: { address?: string; networkPassphrase?: string }): Promise<{ signedMessage: string | Uint8Array; signerAddress?: string; error?: string } | string>;
};
type WalletChallenge = { message: string; nonce: string; expires_at: string };
type GameWasm = { game_state_json(points: number): string };
type ImpactStats = { verified_activities: number; active_participants: number; approved_organizations: number; certificates_issued: number; metrics: { metric: string; unit: string; value: number }[] };
type CommunityGoal = { name: string; metric: string; unit: string; target_value: number; current_value: number };
declare global { interface Window { BarcodeDetector?: new () => BarcodeDetectorLike; solana?: SolanaProvider; freighterApi?: FreighterApi } }

// Backend expects the Ed25519 signature as standard base64.
function toBase64(bytes: Uint8Array) {
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

// Both chains sign with Ed25519, so the server verifies either the same way.
// The address is needed before the challenge, so connecting and signing are separate steps.
async function connectSolana() {
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

async function connectFreighter() {
  const api = window.freighterApi;
  if (!api) throw new Error('No Freighter wallet found. Install the Freighter extension.');
  const granted = await (api.requestAccess?.() ?? api.getAddress?.());
  const address = granted?.address ?? (await api.getPublicKey?.());
  if (!address) throw new Error(granted?.error ?? 'Freighter did not return an address.');
  return {
    address,
    sign: async (text: string) => {
      const signed = await api.signMessage(text, { address });
      // Freighter v2 returns an object; older builds return the base64 string directly.
      if (typeof signed === 'string') return signed;
      if (signed.error) throw new Error(signed.error);
      return typeof signed.signedMessage === 'string' ? signed.signedMessage : toBase64(signed.signedMessage);
    },
  };
}

async function request(path: string, init?: RequestInit) {
  const response = await fetch(`${API_URL}${path}`, { credentials: 'include', ...init });
  if (!response.ok) { const body = await response.json().catch(() => ({})); throw new Error(body.error?.message ?? body.message ?? `Request failed (${response.status})`); }
  return response.status === 204 ? null : response.json();
}

export default function App() {
  const [events, setEvents] = useState<Event[]>([]);
  const [selected, setSelected] = useState<Event>();
  const [message, setMessage] = useState('Loading missions…');
  const [missionsState, setMissionsState] = useState<'loading' | 'ready' | 'error'>('loading');
  const [code, setCode] = useState('');
  const [profile, setProfile] = useState<Profile>();
  const [game, setGame] = useState<GameState>();
  const [gameError, setGameError] = useState('Loading restoration game…');
  const [animation, setAnimation] = useState('');
  const [playerImpact, setPlayerImpact] = useState<ImpactStats>();
  const [platformImpact, setPlatformImpact] = useState<ImpactStats>();
  const [communityGoal, setCommunityGoal] = useState<CommunityGoal>();
  const [achievements, setAchievements] = useState<Achievement[]>([]);
  const [linking, setLinking] = useState(false);
  const [authMode, setAuthMode] = useState<'login' | 'register'>('login');
  const [authForm, setAuthForm] = useState({ username: '', email: '', password: '' });
  const [authBusy, setAuthBusy] = useState(false);
  const wasm = useRef<GameWasm>();
  const gameRef = useRef<GameState>();

  // Session lives in HttpOnly cookies, so the client never holds a token; it
  // only tracks whether /api/auth/me answered.
  async function submitAuth(event: React.FormEvent) {
    event.preventDefault();
    setAuthBusy(true);
    try {
      const path = authMode === 'login' ? '/api/auth/login' : '/api/auth/register';
      const body = authMode === 'login'
        ? { email: authForm.email, password: authForm.password }
        : { username: authForm.username, email: authForm.email, password: authForm.password };
      await request(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
      setAuthForm({ username: '', email: '', password: '' });
      await refreshPlayer();
      setMessage(authMode === 'login' ? 'Signed in.' : 'Account created. You are signed in.');
    } catch (error) { setMessage((error as Error).message); } finally { setAuthBusy(false); }
  }

  async function signOut() {
    try { await request('/api/auth/logout', { method: 'POST' }); } catch { /* clearing local state still signs the user out */ }
    gameRef.current = undefined;
    setProfile(undefined); setGame(undefined); setAchievements([]); setPlayerImpact(undefined); setAnimation('');
    setMessage('Signed out.');
  }

  // Server issues a single-use nonce, the wallet signs it, the server verifies ownership.
  // The private key never leaves the extension and no transaction is submitted.
  async function linkWallet(connect: () => Promise<{ address: string; sign(text: string): Promise<string> }>) {
    setLinking(true);
    try {
      const wallet = await connect();
      const challenge = await request('/api/wallet/challenge', {
        method: 'POST', headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ wallet_address: wallet.address }),
      }) as WalletChallenge;
      const signature = await wallet.sign(challenge.message);
      await request('/api/wallet/verify', {
        method: 'POST', headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ wallet_address: wallet.address, nonce: challenge.nonce, signature }),
      });
      await refreshPlayer();
      setMessage('Wallet ownership verified. Eligible achievements are queued for minting.');
    } catch (error) { setMessage((error as Error).message); } finally { setLinking(false); }
  }

  function refreshImpact() {
    void request('/api/impact').then((stats: ImpactStats) => setPlatformImpact(stats));
    void request('/api/impact/community-goal').then((goal: CommunityGoal) => setCommunityGoal(goal));
    void request('/api/impact/me').then((stats: ImpactStats) => setPlayerImpact(stats)).catch(() => setPlayerImpact(undefined));
  }
  function refreshPlayer() {
    return request('/api/auth/me').then((player: Profile) => {
      const previous = gameRef.current;
      const next = wasm.current ? JSON.parse(wasm.current.game_state_json(player.eco_points)) as GameState : undefined;
      gameRef.current = next; setProfile(player); setGame(next); refreshImpact();
      void request('/api/achievements/me').then((items: Achievement[]) => setAchievements(items)).catch(() => setAchievements([]));
      if (next && previous && (next.level > previous.level || next.character !== previous.character || next.environment !== previous.environment)) setAnimation(`Level ${next.level}: ${next.character} in ${next.environment} unlocked!`);
    }).catch(() => undefined);
  }

  // Returns the promise so callers can refresh the list before reporting their
  // own outcome, instead of having their message overwritten by this one.
  function loadEvents() {
    setMissionsState('loading'); setMessage('Loading missions…');
    return request('/api/events').then((items: Event[]) => { setEvents(items); setMissionsState('ready'); setMessage(items.length ? '' : 'No upcoming missions.'); }).catch((error: Error) => { setMissionsState('error'); setMessage(error.message); });
  }

  useEffect(() => {
    void loadEvents();
    refreshImpact();
    import(/* @vite-ignore */ GAME_WASM_URL).then(async module => { await module.default(); wasm.current = module as GameWasm; setGameError(''); return refreshPlayer(); }).catch(() => setGameError('Restoration game unavailable. Reload after WebAssembly is available.'));
    // API URL is deployment constant; mount-only game initialization is intentional.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function showEvent(id: string) {
    try { setSelected(await request(`/api/events/${id}`) as Event); setMessage(''); } catch (error) { setMessage((error as Error).message); }
  }
  async function join() {
    if (!selected) return;
    try {
      await request(`/api/events/${selected.id}/join`, { method: 'POST' });
      // Refresh the open mission and the list counts first: both clear the
      // status line, so the confirmation has to be set last to survive.
      await showEvent(selected.id);
      await loadEvents();
      setMessage('Joined mission. Check in when the event is active.');
    } catch (error) { setMessage((error as Error).message); }
  }
  async function checkIn(value = code) {
    if (!value.trim()) return setMessage('Enter check-in code.');
    try { await request('/api/check-in', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ code: value.trim() }) }); setCode(''); await refreshPlayer(); setMessage('Check-in recorded. Organizer verification updates restoration after player refresh.'); } catch (error) { setMessage((error as Error).message); }
  }
  async function scanImage(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file || !window.BarcodeDetector) return;
    try { const values = await new window.BarcodeDetector().detect(await createImageBitmap(file)); if (!values[0]?.rawValue) throw new Error('No QR code found.'); setCode(values[0].rawValue); } catch (error) { setMessage((error as Error).message); }
  }

  return <main>
    <header><h1>EcoQuest missions</h1><p>Join local environmental action.</p></header>
    {message && <p role="status">{message}</p>}
    <section className="account" aria-label="Account">
      {profile
        ? <><p>Signed in as {profile.username}.</p><button type="button" onClick={() => void signOut()}>Sign out</button></>
        : <>
          <h2>{authMode === 'login' ? 'Sign in' : 'Create account'}</h2>
          <p>Joining a mission and checking in need an account.</p>
          <form onSubmit={event => void submitAuth(event)}>
            {authMode === 'register' && <label>Username<input value={authForm.username} autoComplete="username" required minLength={3} maxLength={32} onChange={event => setAuthForm({ ...authForm, username: event.target.value })} /></label>}
            <label>Email<input type="email" value={authForm.email} autoComplete="email" required onChange={event => setAuthForm({ ...authForm, email: event.target.value })} /></label>
            <label>Password<input type="password" value={authForm.password} autoComplete={authMode === 'login' ? 'current-password' : 'new-password'} required minLength={12} onChange={event => setAuthForm({ ...authForm, password: event.target.value })} /></label>
            <button type="submit" disabled={authBusy}>{authBusy ? 'Working…' : authMode === 'login' ? 'Sign in' : 'Create account'}</button>
          </form>
          <button type="button" onClick={() => { setAuthMode(authMode === 'login' ? 'register' : 'login'); setMessage(''); }}>
            {authMode === 'login' ? 'Need an account? Register' : 'Already registered? Sign in'}
          </button>
        </>}
    </section>
    <section className={`game stage-${game?.restoration_stage ?? 0}`} aria-label="Restoration game" aria-live="polite">
      <h2>Restoration world</h2>
      {gameError ? <p role="status">{gameError}</p> : game && <><div className="scene" aria-label={['Polluted', 'Cleanup started', 'Recovering', 'Healthy ecosystem'][game.restoration_stage]}><span className="sun">☀</span><span className="character">{game.character === 'Turtle' ? '🐢' : game.character === 'Eco Guardian' ? '🧑‍🌾' : '🧍'}</span><span className="nature">{game.restoration_stage === 3 ? '🌳 🐟 🦋' : game.restoration_stage === 2 ? '🌱 🐟' : game.restoration_stage === 1 ? '🗑️ 🌱' : '🗑️ 🛢️'}</span></div><p>{profile?.username}: {profile?.eco_points} Eco Points · Level {game.level} · {game.character} · {game.environment}</p><progress value={game.progress_points} max={game.progress_points + game.points_to_next_level || 1} /><span>{game.points_to_next_level ? `${game.points_to_next_level} points to next level` : 'Maximum level reached'}</span></>}
      <button type="button" onClick={() => void refreshPlayer()} disabled={Boolean(gameError)}>Refresh player data</button>
      {animation && <p className="unlock" role="status">{animation}</p>}
    </section>
    <section className="impact" aria-label="Blockchain achievements"><h2>OCEAN GUARDIAN</h2>{profile?.wallet_address ? <p>Wallet: {profile.wallet_address}</p> : <><p>Link a wallet to claim achievements once eligible. Signing proves ownership only; it never moves funds.</p><button type="button" onClick={() => void linkWallet(connectFreighter)} disabled={linking}>{linking ? 'Waiting for wallet…' : 'Connect Freighter (Stellar)'}</button><button type="button" onClick={() => void linkWallet(connectSolana)} disabled={linking}>{linking ? 'Waiting for wallet…' : 'Connect Solana wallet'}</button></>}{achievements.map(item => <p key={item.achievement_key}>{item.achievement_key}: {item.status} · ref {item.verification_reference}{item.explorer_url && <a href={item.explorer_url}>Verify on chain</a>}</p>)}</section>
    <section className="impact" aria-label="Impact dashboards">
      <h2>EcoQuest impact</h2>
      {platformImpact && <p>{platformImpact.verified_activities} verified activities · {platformImpact.active_participants} active participants · {platformImpact.approved_organizations} approved organizations · {platformImpact.certificates_issued} certificates issued</p>}
      {platformImpact && <ul>{platformImpact.metrics.map(metric => <li key={`${metric.metric}-${metric.unit}`}>{metric.value} {metric.unit} {metric.metric.replace(/_/g, ' ')}</li>)}</ul>}
      <h3>My impact</h3>
      {playerImpact ? <><p>{playerImpact.verified_activities} verified activities · {playerImpact.certificates_issued} certificates issued</p><ul>{playerImpact.metrics.map(metric => <li key={`${metric.metric}-${metric.unit}`}>{metric.value} {metric.unit} {metric.metric.replace(/_/g, ' ')}</li>)}</ul></> : <p>Sign in to view personal verified impact.</p>}
      {communityGoal && <><h3>Community goal: {communityGoal.name}</h3><progress value={Math.min(communityGoal.current_value, communityGoal.target_value)} max={communityGoal.target_value} /><span>{communityGoal.current_value} / {communityGoal.target_value} {communityGoal.unit}</span></>}
    </section>
    <section aria-label="Available missions" className="missions" aria-busy={missionsState === 'loading'}>
      {missionsState === 'loading' && <p role="status">Loading missions…</p>}
      {missionsState === 'error' && <button type="button" onClick={() => void loadEvents()}>Retry missions</button>}
      {missionsState === 'ready' && events.map(event => <button className="mission" key={event.id} onClick={() => showEvent(event.id)}><strong>{event.name}</strong><span>{event.location} · {new Date(event.starts_at).toLocaleString()}</span><span>{event.registered_count}/{event.capacity} joined · {event.eco_points} points</span></button>)}
    </section>
    {selected && <section className="detail" aria-live="polite"><button onClick={() => setSelected(undefined)}>Close</button><h2>{selected.name}</h2><p>{selected.description}</p><p>{selected.activity_type} · {selected.location}</p><p>{new Date(selected.starts_at).toLocaleString()} to {new Date(selected.ends_at).toLocaleString()}</p><ul>{selected.impacts.map(impact => <li key={`${impact.metric}-${impact.unit}`}>{impact.expected_value} {impact.unit} {impact.metric}</li>)}</ul><button onClick={join}>Join mission</button></section>}
    <section className="checkin"><h2>Check in</h2><label>QR code<input value={code} onChange={event => setCode(event.target.value)} autoComplete="off" /></label><button type="button" onClick={() => void checkIn()}>Check in</button>{window.BarcodeDetector ? <label>Scan QR with camera or image<input type="file" accept="image/*" capture="environment" onChange={scanImage} /></label> : <p>Camera QR scanning is unavailable here. Enter code manually.</p>}</section>
  </main>;
}
