#!/usr/bin/env bash
#
# End-to-end acceptance walk of the documented user flow: normal user, the
# organization request and approval, the organization-owner event lifecycle,
# admin moderation, the authorization matrix, check-in edge cases, and wallet
# ownership. Prints PASS/FAIL per item and exits non-zero if anything fails.
#
# Development/test only: it promotes an account to ADMIN directly in the
# database, because there is deliberately no API to grant yourself that role.
#
# Requires a running API (make api), the docker compose database (make db-up),
# python with `cryptography` (for the Ed25519 wallet proof), and curl.
#
#   bash scripts/verify-user-flow.sh
#
# Note: the auth rate limiter allows AUTH_RATE_LIMIT_MAX logins per window per
# IP. Back-to-back runs will trip it; leave a window between them.
set -u
API=${ECOQUEST_API_URL:-http://localhost:8080}
S=$RANDOM$RANDOM
# A relative scratch dir, not mktemp: under Git Bash on Windows a /tmp path is
# not resolvable by the native python this script shells out to.
J=./.verify-run; rm -rf $J; mkdir -p $J; trap 'rm -rf "$J"' EXIT
PSQL="docker exec ecoquest-postgres psql -U ecoquest -d ecoquest -tAc"
PASS=0; FAIL=0
req() { local jar=$1 m=$2 p=$3 b=${4-}
  if [ -n "$b" ]; then curl -s -o "$J/out" -w "%{http_code}" -b "$J/$jar" -c "$J/$jar" -X "$m" -H 'Content-Type: application/json' -d "$b" "$API$p"
  else curl -s -o "$J/out" -w "%{http_code}" -b "$J/$jar" -c "$J/$jar" -X "$m" "$API$p"; fi; }
jget() { python -c "import json;print(json.load(open('$J/out')).get('$1',''))" 2>/dev/null; }
ck() { # ck <label> <expected-code> <actual-code>
  if [ "$2" = "$3" ]; then PASS=$((PASS+1)); printf '  PASS  %-58s %s\n' "$1" "$3"
  else FAIL=$((FAIL+1)); printf '  FAIL  %-58s got %s want %s | %s\n' "$1" "$3" "$2" "$(head -c 160 "$J/out")"; fi; }
ckt() { # ckt <label> <true|false>
  if [ "$2" = "True" ] || [ "$2" = "true" ]; then PASS=$((PASS+1)); printf '  PASS  %s\n' "$1"
  else FAIL=$((FAIL+1)); printf '  FAIL  %s\n' "$1"; fi; }
reg() { req "$1" POST /api/auth/register "{\"username\":\"$1$S\",\"email\":\"$1$S@t.test\",\"password\":\"Str0ng!Passw0rd\"}"; }
ts() { python -c "import datetime as d;print((d.datetime.now(d.timezone.utc)+d.timedelta(hours=$1)).strftime('%Y-%m-%dT%H:%M:%SZ'))"; }

echo "=== NORMAL USER ==="
ck "register a new account"                201 "$(reg player)"
ckt "account is created as a normal USER"  "$(python -c "import json;print(json.load(open('$J/out'))['user']['role']=='USER')")"
ck "logout"                                204 "$(req player POST /api/auth/logout)"
ck "log in"                                200 "$(req player POST /api/auth/login "{\"email\":\"player$S@t.test\",\"password\":\"Str0ng!Passw0rd\"}")"
ck "session refresh"                       200 "$(req player POST /api/auth/refresh)"
ck "open the user dashboard (impact/me)"   200 "$(req player GET /api/impact/me)"
ck "browse missions"                       200 "$(req player GET /api/events)"

echo
echo "=== ORGANIZATION REQUEST ==="
reg owner >/dev/null; reg admin >/dev/null
$PSQL "UPDATE users SET role='ADMIN' WHERE email='admin$S@t.test'" >/dev/null
req admin POST /api/auth/login "{\"email\":\"admin$S@t.test\",\"password\":\"Str0ng!Passw0rd\"}" >/dev/null
ck "submit an organization request"        201 "$(req owner POST /api/organizations "{\"name\":\"Org $S\",\"organization_type\":\"NGO\",\"location\":\"B\",\"description\":\"d\"}")"
ORG=$(jget id)
ckt "organization is shown as PENDING"     "$(python -c "import json;print(json.load(open('$J/out'))['verification_status']=='PENDING')")"
ST=$(ts -1); EN=$(ts 3)
EVBODY="{\"name\":\"M$S\",\"description\":\"d\",\"activity_type\":\"BEACH_CLEANUP\",\"location\":\"B\",\"starts_at\":\"$ST\",\"ends_at\":\"$EN\",\"capacity\":10,\"eco_points\":600,\"impacts\":[{\"metric\":\"waste_collected\",\"unit\":\"kg\",\"expected_value\":4}]}"
ck "PENDING org owner cannot create events" 403 "$(req owner POST "/api/organizations/$ORG/events" "$EVBODY")"
ck "normal user cannot list organizations (admin)" 403 "$(req player GET /api/admin/organizations)"
ck "admin views all organizations"         200 "$(req admin GET /api/admin/organizations)"
ck "admin reviews the pending organization" 200 "$(req admin GET "/api/admin/organizations/$ORG")"
ck "admin approves the organization"       200 "$(req admin PATCH "/api/admin/organizations/$ORG/status" '{"status":"APPROVED","reason":"ok"}')"
ckt "organization becomes APPROVED"        "$(python -c "import json;print(json.load(open('$J/out'))['verification_status']=='APPROVED')")"
ck "same user logs back in (no 2nd account)" 200 "$(req owner POST /api/auth/login "{\"email\":\"owner$S@t.test\",\"password\":\"Str0ng!Passw0rd\"}")"
ck "owner now sees the org dashboard"      200 "$(req owner GET "/api/organizations/$ORG/events")"

echo
echo "=== ORGANIZATION OWNER ==="
ck "create an event (draft)"               201 "$(req owner POST "/api/organizations/$ORG/events" "$EVBODY")"
EV=$(jget id)
ck "edit the event as a draft"             200 "$(req owner PUT "/api/events/$EV" "${EVBODY/M$S/M$S edited}")"
ck "publish the event"                     204 "$(req owner POST "/api/events/$EV/publish")"
req player GET /api/events >/dev/null
ckt "published event appears in discovery" "$(python -c "import json;print(any(e['id']=='$EV' for e in json.load(open('$J/out'))))")"
ck "activate the event"                    204 "$(req owner POST "/api/events/$EV/activate")"
req player GET /api/events >/dev/null
ckt "RUNNING event still appears in discovery" "$(python -c "import json;print(any(e['id']=='$EV' for e in json.load(open('$J/out'))))")"
ck "generate the event QR code"            200 "$(req owner POST "/api/events/$EV/qr" '{}')"
CODE=$(jget code)
ck "user joins the running mission"        201 "$(req player POST "/api/events/$EV/join")"
req player GET /api/me/activities >/dev/null
ckt "mission appears under My Activities"  "$(python -c "import json;print(any(a['event']['id']=='$EV' for a in json.load(open('$J/out'))))")"
ck "complete the QR check-in"              200 "$(req player POST /api/check-in "{\"code\":\"$CODE\"}")"
ckt "participation enters PENDING_VERIFICATION" "$(python -c "import json;print(json.load(open('$J/out'))['status']=='PENDING_VERIFICATION')")"
ck "open the participant list"             200 "$(req owner GET "/api/events/$EV/participants")"
PID=$(python -c "import json;print(json.load(open('$J/out'))[0]['id'])" 2>/dev/null)
ck "verify the participant"                200 "$(req owner POST "/api/events/$EV/participants/verify" "{\"participation_ids\":[\"$PID\"],\"reason\":\"attended\"}")"
ckt "verification awards the expected Eco Points" "$(python -c "import json;print(json.load(open('$J/out'))['results'][0]['points_awarded']==600)")"
sleep 3
req player GET /api/auth/me >/dev/null
ckt "Eco Points ledger reflects the award" "$(python -c "import json;print(json.load(open('$J/out'))['eco_points']==600)")"
req player GET /api/achievements/me >/dev/null
ckt "achievements update (FIRST_STEPS earned)" "$(python -c "import json;a=json.load(open('$J/out'));print(any(x['achievement_key']=='FIRST_STEPS' and x['earned'] for x in a))")"
ckt "600 pts earns POINT_COLLECTOR (>=500)" "$(python -c "import json;a=json.load(open('$J/out'));print(any(x['achievement_key']=='POINT_COLLECTOR' and x['earned'] for x in a))")"
ckt "next tier shows partial progress (600/2500)" "$(python -c "import json;a=json.load(open('$J/out'));print(any(x['achievement_key']=='ECO_CHAMPION' and x['progress']==600 and x['threshold']==2500 and not x['earned'] for x in a))")"
ckt "on-chain OCEAN_GUARDIAN becomes eligible" "$(python -c "import json;a=json.load(open('$J/out'));print(any(x['achievement_key']=='OCEAN_GUARDIAN' and x['kind']=='ONCHAIN' and x['earned'] for x in a))")"
req player GET /api/impact/me >/dev/null
ckt "impact tracking updates"              "$(python -c "import json;d=json.load(open('$J/out'));print(d['verified_activities']==1 and any(m['metric']=='waste_collected' for m in d['metrics']))")"
req player GET /api/certificates/me >/dev/null
ckt "certificate becomes available"        "$(python -c "import json;print(any(c['participation_id']=='$PID' for c in json.load(open('$J/out'))))")"

echo
echo "=== ADMIN ==="
ck "admin views events"                    200 "$(req admin GET /api/admin/events)"
ck "admin opens event details"             200 "$(req admin GET "/api/admin/events/$EV")"
ST2=$(ts 1); EN2=$(ts 4)
req owner POST "/api/organizations/$ORG/events" "{\"name\":\"C$S\",\"description\":\"d\",\"activity_type\":\"RECYCLING\",\"location\":\"B\",\"starts_at\":\"$ST2\",\"ends_at\":\"$EN2\",\"capacity\":5,\"eco_points\":50,\"impacts\":[]}" >/dev/null
EVC=$(jget id)
req owner POST "/api/events/$EVC/publish" >/dev/null
ck "admin cancels an event with a reason"  200 "$(req admin POST "/api/admin/events/$EVC/cancel" '{"reason":"weather advisory"}')"
req admin GET "/api/admin/events/$EVC" >/dev/null
ckt "cancellation state + reason recorded" "$(python -c "import json;e=json.load(open('$J/out'))['event'];print(e['status']=='CANCELLED' and e['cancellation_reason']=='weather advisory')")"
ck "cancelled event is no longer joinable" 409 "$(req player POST "/api/events/$EVC/join")"
req player GET /api/events >/dev/null
ckt "cancelled event leaves discovery"     "$(python -c "import json;print(not any(e['id']=='$EVC' for e in json.load(open('$J/out'))))")"
# Rejection applies to a PENDING application; an approved org is suspended, not rejected.
reg owner2 >/dev/null
req owner2 POST /api/organizations "{\"name\":\"Org2 $S\",\"organization_type\":\"NGO\",\"location\":\"B\",\"description\":\"d\"}" >/dev/null
ORG2=$(jget id)
ck "admin rejects a pending organization"  200 "$(req admin PATCH "/api/admin/organizations/$ORG2/status" '{"status":"REJECTED","reason":"incomplete"}')"
ckt "rejected status reflected"            "$(python -c "import json;print(json.load(open('$J/out'))['verification_status']=='REJECTED')")"
ck "approved org cannot be rejected outright (suspend instead)" 409 "$(req admin PATCH "/api/admin/organizations/$ORG/status" '{"status":"REJECTED","reason":"x"}')"

echo
echo "=== AUTHORIZATION ==="
ck "rejected org owner cannot create events" 403 "$(req owner2 POST "/api/organizations/$ORG2/events" "$EVBODY")"
req admin PATCH "/api/admin/organizations/$ORG/status" '{"status":"SUSPENDED","reason":"probe"}' >/dev/null
ck "suspended org cannot issue a check-in code" 403 "$(req owner POST "/api/events/$EV/qr" '{}')"
req admin PATCH "/api/admin/organizations/$ORG/status" '{"status":"APPROVED","reason":"ok"}' >/dev/null
ck "normal user cannot manage another user's org" 403 "$(req player GET "/api/organizations/$ORG/events")"
ck "normal user cannot create org events"  403 "$(req player POST "/api/organizations/$ORG/events" "$EVBODY")"
ck "normal user cannot verify participants" 403 "$(req player POST "/api/events/$EV/participants/verify" "{\"participation_ids\":[\"$PID\"],\"reason\":\"x\"}")"
ck "normal user cannot reach admin users"  403 "$(req player GET /api/admin/users)"
ck "normal user cannot cancel via admin"   403 "$(req player POST "/api/admin/events/$EV/cancel" '{"reason":"x"}')"
ck "anonymous cannot read activities"      401 "$(req anon GET /api/me/activities)"

echo
echo "=== CHECK-IN EDGE CASES ==="
ck "QR window that misses the mission is refused" 400 "$(req owner POST "/api/events/$EV/qr" "{\"activates_at\":\"$(ts 30)\",\"expires_at\":\"$(ts 34)\"}")"
echo "        reason: $(head -c 220 "$J/out")"
ck "unknown check-in code"                 404 "$(req player POST /api/check-in '{"code":"ZZZZ-ZZZZ"}')"
ck "organizer rotates the code"            200 "$(req owner POST "/api/events/$EV/qr" '{}')"
CODE2=$(jget code)
ck "the rotated-away code stops working"   409 "$(req player POST /api/check-in "{\"code\":\"$CODE\"}")"
echo "        reason: $(head -c 160 "$J/out")"
ck "already-verified player re-scans"      409 "$(req player POST /api/check-in "{\"code\":\"$CODE2\"}")"
echo "        reason: $(head -c 220 "$J/out")"
reg nojoin >/dev/null
ck "never-joined player scans"             409 "$(req nojoin POST /api/check-in "{\"code\":\"$CODE2\"}")"
echo "        reason: $(head -c 220 "$J/out")"

echo
echo "=== WALLET OWNERSHIP ==="
# Per-run keypair: users.wallet_address is uniquely indexed, so a fixed key
# would collide with the previous run.
SEED=$(python -c "import hashlib;print(hashlib.sha256(b'$S').hexdigest())")
W=$(python -c "
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization
pub=Ed25519PrivateKey.from_private_bytes(bytes.fromhex('$SEED')).public_key().public_bytes(
    serialization.Encoding.Raw, serialization.PublicFormat.Raw)
A='123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'
n=int.from_bytes(pub,'big'); s=''
while n: n,r=divmod(n,58); s=A[r]+s
print('1'*(len(pub)-len(pub.lstrip(b'\x00')))+s)")
sign() { python -c "
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
import base64,sys
k=Ed25519PrivateKey.from_private_bytes(bytes.fromhex('$SEED'))
msg=f'EcoQuest wallet ownership\nwallet:$W\nnonce:{sys.argv[1]}'.encode()
print(base64.b64encode(k.sign(msg)).decode())" "$1"; }

ck "wallet challenge issued"               200 "$(req player POST /api/wallet/challenge "{\"wallet_address\":\"$W\"}")"
NONCE=$(jget nonce)
ck "forged signature rejected"             403 "$(req player POST /api/wallet/verify "{\"wallet_address\":\"$W\",\"nonce\":\"$NONCE\",\"signature\":\"$(sign wrong-nonce)\"}")"
ck "fresh challenge after a failed attempt" 200 "$(req player POST /api/wallet/challenge "{\"wallet_address\":\"$W\"}")"
NONCE=$(jget nonce)
ck "correctly signed challenge links the wallet" 200 "$(req player POST /api/wallet/verify "{\"wallet_address\":\"$W\",\"nonce\":\"$NONCE\",\"signature\":\"$(sign "$NONCE")\"}")"
sleep 4
req player GET /api/achievements/me >/dev/null
ckt "linked wallet mints the on-chain achievement" "$(python -c "import json;a=json.load(open('$J/out'));print(any(x['kind']=='ONCHAIN' and x['status']=='MINTED' and x['explorer_url'] for x in a))")"

echo
echo "-------------------------------------------"
echo "PASS=$PASS  FAIL=$FAIL"
[ "$FAIL" -eq 0 ]
