param([string]$ApiUrl = 'http://localhost:8080')
$ErrorActionPreference = 'Stop'
if ($env:APP_ENV -notin @('development', 'test')) { throw 'Demo allowed only when APP_ENV=development or APP_ENV=test.' }
$pw = 'Demo-password-1234'
function Call-Api($method, $path, $body, $session) {
  $args = @{ Method=$method; Uri="$ApiUrl$path"; ContentType='application/json'; ErrorAction='Stop' }
  if ($null -ne $body) { $args.Body = $body | ConvertTo-Json -Depth 8 }
  if ($null -ne $session) { $args.WebSession = $session }
  Invoke-RestMethod @args
}
foreach ($user in @(@{username='eco_admin';email='admin@ecoquest.test'}, @{username='eco_organizer';email='organizer@ecoquest.test'}, @{username='alex';email='alex@ecoquest.test'})) {
  try { Call-Api Post '/api/auth/register' @{username=$user.username;email=$user.email;password=$pw} $null | Out-Null } catch { if ($_.Exception.Response.StatusCode.value__ -ne 409) { throw } }
}
Get-Content "$PSScriptRoot/seed-demo.sql" -Raw | docker compose exec -T postgres psql -U ecoquest -d ecoquest -v ON_ERROR_STOP=1
$organizer = New-Object Microsoft.PowerShell.Commands.WebRequestSession
Call-Api Post '/api/auth/login' @{email='organizer@ecoquest.test';password=$pw} $organizer | Out-Null
$event = Call-Api Post '/api/organizations/00000000-0000-0000-0000-000000000010/events' @{name='Butuan Coastal Cleanup';description='Competition demonstration';activity_type='BEACH_CLEANUP';location='Butuan City';starts_at=(Get-Date).ToUniversalTime().AddMinutes(-30).ToString('yyyy-MM-ddTHH:mm:ssZ');ends_at=(Get-Date).ToUniversalTime().AddHours(4).ToString('yyyy-MM-ddTHH:mm:ssZ');capacity=50;eco_points=1000;impacts=@(@{metric='waste_collected';unit='kg';expected_value=12})} $organizer
Call-Api Post "/api/events/$($event.id)/publish" $null $organizer | Out-Null
$alex = New-Object Microsoft.PowerShell.Commands.WebRequestSession
Call-Api Post '/api/auth/login' @{email='alex@ecoquest.test';password=$pw} $alex | Out-Null
$join = Call-Api Post "/api/events/$($event.id)/join" $null $alex
Call-Api Post "/api/events/$($event.id)/activate" $null $organizer | Out-Null
$qr = Call-Api Post "/api/events/$($event.id)/qr" @{} $organizer
Call-Api Post '/api/check-in' @{code=$qr.code} $alex | Out-Null
$verified = Call-Api Post "/api/events/$($event.id)/participants/verify" @{participation_ids=@($join.id);reason='Demo verification'} $organizer
if ($verified.results[0].points_awarded -ne 1000 -or $verified.results[0].player_eco_points -ne 1000) { throw 'Expected Alex to receive exactly 1000 points.' }
$proof = "SELECT (SELECT eco_points FROM users WHERE lower(email)='alex@ecoquest.test')=1000 AS points_exact, (SELECT COALESCE(sum(value),0) FROM impact_contributions WHERE event_id='$($event.id)' AND metric='waste_collected')=12 AS waste_exact, EXISTS(SELECT 1 FROM blockchain_achievements b JOIN users u ON u.id=b.user_id WHERE lower(u.email)='alex@ecoquest.test' AND b.achievement_key='OCEAN_GUARDIAN') AS ocean_guardian, EXISTS(SELECT 1 FROM outbox_events WHERE aggregate_id='$($join.id)' AND event_type='certificate.issue') AS certificate_queued;"
$proofResult = (docker compose exec -T postgres psql -U ecoquest -d ecoquest -tAc $proof).Trim()
if ($proofResult -ne 't|t|t|t') { throw "Demo database assertions failed: $proofResult" }
Write-Output 'Demo passed: approved organization, event, QR check-in, verification, exactly 1,000 points, 12 kg impact, Ocean Guardian eligibility, certificate issuance queued.'
Write-Output 'Fallback: use docs/demo-fallback.md when camera or blockchain wallet signing is unavailable.'
