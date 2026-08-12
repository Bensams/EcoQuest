# PostgreSQL backup and restore

Run backups from trusted operator machine. Backup file contains user and impact data. Encrypt at rest, restrict access, and test restore quarterly.

## Backup

```powershell
docker compose exec -T postgres pg_dump -U ecoquest -d ecoquest --format=custom --file=/tmp/ecoquest.dump
docker cp ecoquest-postgres:/tmp/ecoquest.dump .\ecoquest-$(Get-Date -Format yyyyMMdd-HHmmss).dump
```

Store encrypted copy outside Docker host. Retain per policy. Do not put backups in Git.

## Restore drill

This destroys target database. Confirm target is isolated development/staging and backup is correct.

```powershell
docker cp .\ecoquest-YYYYMMDD-HHMMSS.dump ecoquest-postgres:/tmp/restore.dump
docker compose exec -T postgres dropdb -U ecoquest --if-exists ecoquest
docker compose exec -T postgres createdb -U ecoquest ecoquest
docker compose exec -T postgres pg_restore -U ecoquest -d ecoquest --clean --if-exists /tmp/restore.dump
docker compose exec -T postgres psql -U ecoquest -d ecoquest -c 'SELECT count(*) FROM users;'
```

Start API after restore. SQLx migration history must match deployed binary; never restore a newer schema into an older binary.
