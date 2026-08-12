param([switch]$Force)
$ErrorActionPreference = 'Stop'
if ($env:APP_ENV -notin @('development', 'test')) { throw 'Reset allowed only when APP_ENV=development or APP_ENV=test.' }
if (-not $Force) { throw 'Pass -Force. This destroys local development/test data.' }
docker compose exec -T postgres psql -U ecoquest -d ecoquest -v ON_ERROR_STOP=1 -c 'DROP SCHEMA public CASCADE; CREATE SCHEMA public; GRANT ALL ON SCHEMA public TO ecoquest;'
Write-Output 'Database reset. Start ecoquest-api to apply migrations.'
