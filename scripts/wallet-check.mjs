// Live check: a Stellar StrKey address and a Solana base58 address must both
// pass /api/wallet/challenge + /api/wallet/verify with a real Ed25519 signature.
// Usage: node scripts/wallet-check.mjs [api-url]
const API = process.argv[2] ?? 'http://127.0.0.1:8080';
const PASSWORD = process.env.SEED_PASSWORD ?? 'ChangeMe-Local-1234';

const B32 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ234567';
const B58 = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';

function crc16(data) {
  let crc = 0;
  for (const byte of data) {
    crc ^= byte << 8;
    for (let i = 0; i < 8; i += 1) crc = (crc & 0x8000 ? (crc << 1) ^ 0x1021 : crc << 1) & 0xffff;
  }
  return crc;
}
function strkey(key) {
  const payload = Uint8Array.from([0x30, ...key, crc16([0x30, ...key]) & 0xff, crc16([0x30, ...key]) >> 8]);
  let out = '', buffer = 0, bits = 0;
  for (const byte of payload) {
    buffer = (buffer << 8) | byte; bits += 8;
    while (bits >= 5) { bits -= 5; out += B32[(buffer >> bits) & 31]; }
  }
  return bits ? out + B32[(buffer << (5 - bits)) & 31] : out;
}
function base58(bytes) {
  let n = bytes.reduce((acc, b) => acc * 256n + BigInt(b), 0n), out = '';
  while (n > 0n) { out = B58[Number(n % 58n)] + out; n /= 58n; }
  for (const b of bytes) { if (b !== 0) break; out = '1' + out; }
  return out;
}

async function api(path, body, cookie) {
  const response = await fetch(`${API}${path}`, {
    method: body ? 'POST' : 'GET',
    headers: { 'content-type': 'application/json', ...(cookie ? { cookie } : {}) },
    body: body ? JSON.stringify(body) : undefined,
  });
  const text = await response.text();
  if (!response.ok) throw new Error(`${path} → ${response.status} ${text}`);
  return { data: text ? JSON.parse(text) : null, cookie: response.headers.getSetCookie?.().map(c => c.split(';')[0]).join('; ') };
}

async function login(email) {
  await api('/api/auth/register', { username: email.split('@')[0], email, password: PASSWORD }).catch(() => {});
  return (await api('/api/auth/login', { email, password: PASSWORD })).cookie;
}

async function run(label, email, encode) {
  const pair = await crypto.subtle.generateKey({ name: 'Ed25519' }, true, ['sign', 'verify']);
  const raw = new Uint8Array(await crypto.subtle.exportKey('raw', pair.publicKey));
  const address = encode(raw);
  const cookie = await login(email);
  const { data: challenge } = await api('/api/wallet/challenge', { wallet_address: address }, cookie);
  const signature = new Uint8Array(await crypto.subtle.sign({ name: 'Ed25519' }, pair.privateKey, new TextEncoder().encode(challenge.message)));
  await api('/api/wallet/verify', { wallet_address: address, nonce: challenge.nonce, signature: Buffer.from(signature).toString('base64') }, cookie);
  const { data: me } = await api('/api/auth/me', null, cookie);
  if (me.wallet_address !== address) throw new Error(`${label}: stored ${me.wallet_address} != ${address}`);
  console.log(`${label} OK: ${address}`);

  // A signature from a different key must never link a wallet.
  const other = await crypto.subtle.generateKey({ name: 'Ed25519' }, true, ['sign', 'verify']);
  const { data: second } = await api('/api/wallet/challenge', { wallet_address: address }, cookie);
  const forged = new Uint8Array(await crypto.subtle.sign({ name: 'Ed25519' }, other.privateKey, new TextEncoder().encode(second.message)));
  await api('/api/wallet/verify', { wallet_address: address, nonce: second.nonce, signature: Buffer.from(forged).toString('base64') }, cookie)
    .then(() => { throw new Error(`${label}: forged signature accepted`); }, () => console.log(`${label} OK: forged signature rejected`));
}

await run('stellar', 'stellar-wallet@ecoquest.test', strkey);
await run('solana', 'solana-wallet@ecoquest.test', base58);
