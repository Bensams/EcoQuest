import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

const certDir = fileURLToPath(new URL('./.certs', import.meta.url));
const keyPath = `${certDir}/dev-key.pem`;
const certPath = `${certDir}/dev-cert.pem`;

// Browsers only expose the camera (getUserMedia) in a secure context: HTTPS, or
// localhost as a special case. A plain-HTTP LAN address is neither, so live QR
// scanning from a phone needs the dev server to speak HTTPS.
//
// Enabled by the certificates simply being present. Delete web/.certs to go
// back to plain HTTP. Generate them with:
//   openssl req -x509 -newkey rsa:2048 -nodes -days 825 \
//     -keyout web/.certs/dev-key.pem -out web/.certs/dev-cert.pem \
//     -subj "/CN=EcoQuest Dev" \
//     -addext "subjectAltName=DNS:localhost,IP:127.0.0.1,IP:<your-lan-ip>"
const https =
  existsSync(keyPath) && existsSync(certPath)
    ? { key: readFileSync(keyPath), cert: readFileSync(certPath) }
    : undefined;

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // The API is proxied under the dev server's own origin so session cookies
  // (SameSite=Strict) are always first-party, whether the app is opened on
  // localhost or on a LAN address from a phone. Without this, the client would
  // need an absolute API URL and the cookies would be dropped as cross-site.
  server: {
    port: 5173,
    https,
    proxy: {
      '/api': {
        target: process.env.VITE_PROXY_TARGET ?? 'http://localhost:8080',
        changeOrigin: false,
      },
    },
  },
});
