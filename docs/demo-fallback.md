# Competition demo and fallback

## Live script

1. Copy `.env.example` to `.env`; keep `APP_ENV=development`.
2. Start PostgreSQL and API. API applies migrations.
3. Run `powershell -ExecutionPolicy Bypass -File .\scripts\demo.ps1`.
4. Show output assertions: approved Butuan Environmental Organization, Butuan Coastal Cleanup, QR check-in, Alex exactly 1,000 points, 12 kg waste, Ocean Guardian eligibility, certificate issue queued.
5. Sign in as Alex in web client. At 1,000 points game reaches level 11, Ocean environment, restoration stage 2. Sea Turtle unlocks at 400 points.

## Camera fallback

Use organizer-generated QR `code` in check-in field. Mobile `<input capture="environment">` opens rear camera when browser supports `BarcodeDetector`; image picker/manual entry remain fallback. Capture a screenshot of QR SVG and result before judging.

## Blockchain fallback

Current adapter is deterministic `mock://solana/...`, not Solana issuance. Show Ocean Guardian `ELIGIBLE` state and opaque reference. Do not claim real blockchain issuance. Use a screenshot of this state if wallet signing/network fails.

## Certificate fallback

Current verification atomically queues `certificate.issue`; no worker/download/public verifier exists. Show queued outbox proof from demo output. Do not claim downloadable or public certificate verification. Screenshot this limitation and `docs/api.md` missing-contract note.

## Reset

`$env:APP_ENV='development'; powershell -ExecutionPolicy Bypass -File .\scripts\reset-demo.ps1 -Force`

Reset destroys all local data. Restart API, then run demo.
