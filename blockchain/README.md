# Solana achievement credential

Phase 8 backend records one opaque `OCEAN_GUARDIAN` verification reference. Never write name, email, location, user UUID, event ID, or impact details on-chain.

## Local proof prerequisite

This workstation has no `solana`, `solana-test-validator`, or `anchor` executable. Backend runs deterministic `MockBlockchainAdapter` until local signer/program tooling is installed. It stores mock mint/transaction identifiers only; these are not Solana transactions.

Install Solana CLI, then start validator before changing adapter to a signer-backed localnet adapter:

```powershell
solana-test-validator --reset
solana config set --url http://127.0.0.1:8899
solana cluster-version
```

Do not deploy devnet until same transaction is verified against local validator. Explorer links use `https://explorer.solana.com/tx/<signature>?cluster=custom&customUrl=http%3A%2F%2F127.0.0.1%3A8899`; devnet replaces cluster with `devnet`.

Backend guarantees: database trigger grants `ELIGIBLE` after configured verified `BEACH_CLEANUP` count; wallet signed challenge queues it; outbox worker transitions `QUEUED`, `MINTING`, `MINTED`, or `FAILED`; retries are 1, 2, 4, 8 minutes then terminal failure. Event verification, points, certificates, and impact never roll back.
