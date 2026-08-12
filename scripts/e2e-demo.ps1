# End-to-end automated workflow check. API and migrated local PostgreSQL must be running.
$ErrorActionPreference = 'Stop'
& "$PSScriptRoot/demo.ps1" @args
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
