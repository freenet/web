# Notes on API setup

WARNING: This file is publicly readable, do NOT put anything secret in here.

## Where it runs

gkapi runs on **nova** as `gkapi.service` (unit: `/etc/systemd/system/gkapi.service`,
binary: `/home/gkapi/bin/ghostkey-api`, user `gkapi`). It was migrated from vega on
2026-08-28. Caddy terminates TLS for `gkapi.freenet.org` and owns ACME (see
`/etc/caddy/gkapi.caddy`), and proxies to the API on `:8081`. So the binary binds an
unprivileged port and needs no file capability. The unit file's comments explain why the
TLS flags and `setcap` are gone and why the ufw `deny 8081` rule must stay.

## Deploying gkapi

There is **no CI deployment for this crate**. `deploy.yml` builds the Hugo site and
publishes it to GitHub Pages; it never touches the API. `rust-api-tests.yml` only runs
fmt, build and test. Merging a change to `rust/api` therefore ships nothing: the binary
has to be replaced by hand, and has in the past sat months behind main.

```bash
# 1. build on nova, from main, and prove the build contains what you expect
cd ~/code/freenet/web/main/rust && cargo build --release -p ghostkey-api
strings target/release/ghostkey-api | grep -c payment_claim   # sanity: expect non-zero

# 2. stage it next to the live binary, then swap with two adjacent renames
sudo install -o gkapi -g gkapi -m 755 target/release/ghostkey-api /home/gkapi/bin/ghostkey-api.staged
STAMP=$(date +%Y%m%d-%H%M%S)
sudo mv /home/gkapi/bin/ghostkey-api        /home/gkapi/bin/ghostkey-api.rollback-$STAMP
sudo mv /home/gkapi/bin/ghostkey-api.staged /home/gkapi/bin/ghostkey-api
echo "rollback binary: /home/gkapi/bin/ghostkey-api.rollback-$STAMP"
sudo systemctl restart gkapi
```

Do not `mv` the old binary away in one command and copy the new one in with a later one.
Between those two the path does not exist, and anything that restarts the unit in that
window fails with `status=203/EXEC` and stays down.

### Verifying a deploy

```bash
systemctl is-active gkapi
curl -s https://gkapi.freenet.org/                       # {"message":"Hello, world!"}
```

Then confirm every donation tier still resolves its notary keypair, which is a separate
failure mode from the binary (see the tier comment in
`hugo-site/themes/freenet/layouts/shortcodes/stripe-donation-form.html`).

Note that `/create-donation` has no dry-run mode: each call creates a real PaymentIntent in
the live Stripe account. Nothing is charged and no card is attached, so these are harmless
abandoned intents, exactly what a visitor clicking between the amount radios produces. Run
the loop once after a deploy; do not wrap it in a retry-until-success script.

```bash
for a in 1 5 20 50 100 500 2500 10000; do
  curl -s -X POST https://gkapi.freenet.org/create-donation \
    -H 'Content-Type: application/json' -d "{\"amount\":$((a*100)),\"currency\":\"usd\"}" \
  | grep -q notary_certificate_base64 && echo "\$$a ok" || echo "\$$a FAIL"
done
```

### Rollback

Substitute `<stamp>` with the timestamp the deploy printed.

```bash
sudo mv /home/gkapi/bin/ghostkey-api.rollback-<stamp> /home/gkapi/bin/ghostkey-api
sudo systemctl restart gkapi
```

**Once a notary schedule month is live, do not roll back to a binary that predates the
schedule** (anything before freenet/web#192). That binary ignores `notary_period` and signs
with the flat files, so every donation quoted from a month directory and not yet signed is
charged and gets a signature that does not unblind. The signing call succeeds, so the
PaymentIntent stays marked `certificate_signed` and the donor cannot retry. If it happens
anyway, roll forward, then clear `certificate_signed` only on the PaymentIntents of donors
who report that key generation failed (the success page shows the error; the PaymentIntent
id is in its URL), then tell each of them to reload that success page. Do not clear it in bulk on every PaymentIntent
with `notary_period`: the server cannot tell which signatures failed to unblind, and a
donor whose key worked would be able to mint a second one.

## Notary keys and the monthly schedule

The notary directory (`--notary-dir`, `/home/gkapi/delegate-keys` on nova) holds one
keypair per donation tier: `notary_certificate_<amount>.pem` and
`notary_signing_key_<amount>.pem` (older deployments use `delegate_*` names, which are
still read). The certificate's info string carries the amount and a date,
`delegate-key-created`, which every ghost key issued from it shows publicly.

To keep that date current without taking the master key out of storage every month, the
directory can also hold a schedule: one subdirectory per month, `YYYY-MM/`, each a complete
set of tiers.

### How the API uses it

- **Quoting** (`/create-donation`, `/update-donation`) uses the newest `YYYY-MM/` that is not
  in the future, or the flat files if there is none, and records which on the PaymentIntent
  as `notary_period` metadata. If the current month is missing it logs an error and keeps
  issuing from the newest earlier month, so a schedule that runs out degrades to a stale
  date, not an outage. If the chosen month lacks a tier, quoting that tier fails, before
  any charge.
- **Blinding** happens on the success page after the charge, possibly in a later month.
  The page fetches the certificate its PaymentIntent was quoted
  (`GET /notary-certificate/{payment_intent_id}`) rather than trusting localStorage, which
  every tab shares.
- **Signing** (`/sign-certificate`) uses exactly the pair the PaymentIntent was quoted
  from, never whatever is current by then, and refuses (409, before marking the payment
  spent) if the client says it blinded against a different certificate. A PaymentIntent
  with no `notary_period` (quoted from the flat files, including everything before the
  schedule existed) signs with the flat files.

So a month directory, and the flat files, must never be **deleted or replaced** while a
donation quoted from it could still be in checkout: that breaks the donation after the
charge. Keep past months; they are small. Never regenerate into a live directory.

### Generating (master key mounted, one session)

```bash
rust/cli/generate_notary_keys.sh --master-key <master_signing_key.pem> \
  --notary-dir <drive>/notary-schedule --start-month 2026-10 --months 120
```

By default $1 and $5 are dated monthly, and $20 and up are dated quarterly (1 January,
April, July or October).
- **Why not monthly for everything:** the date partitions each tier's anonymity set, and the
  higher tiers see too few donors a month for a monthly date to be safe (see the script
  header).
- **Why not yearly:** an application that wants a *recent* ghost key can still use the date.
  While the schedule covers the current month, a key is never shown more than three months
  older than it is.

The script verifies every pair against the compiled-in Freenet master key. It builds each
month in a hidden directory before renaming it into place, and refuses to touch an existing
month. When extending a schedule mid-quarter, it reuses that quarter's pairs, after
checking them. `rust/cli/test_notary_schedule.sh` tests it (CI runs it).
To extend later, run it again with `--start-month` just after the last month.

Keep the output on the encrypted master-key drive. It is the backup.

### Installing on nova: a rolling window

Do not copy the whole schedule into the API's directory. If the API process were
compromised, every future month's notary key would leak with it, and forged ghost keys
dated years ahead would be indistinguishable from real ones. Instead keep the schedule
root-only and let a daily timer publish the current and next month:

```bash
# once, from the mounted drive
sudo install -d -m 700 -o root -g root /var/lib/gkapi-notary-schedule
sudo cp -a <drive>/notary-schedule/20??-?? /var/lib/gkapi-notary-schedule/
sudo chown -R root:root /var/lib/gkapi-notary-schedule
sudo install -m 755 rust/api/publish_notary_window.sh /usr/local/sbin/publish-notary-window
```

`/etc/systemd/system/gkapi-notary-window.service`:

```ini
[Unit]
Description=Publish the next months of the ghost key notary schedule to gkapi

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/publish-notary-window /var/lib/gkapi-notary-schedule /home/gkapi/delegate-keys gkapi:gkapi 1
```

`/etc/systemd/system/gkapi-notary-window.timer`:

```ini
[Unit]
Description=Daily ghost key notary schedule publish

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl start gkapi-notary-window.service   # publish now; check its output
sudo systemctl enable --now gkapi-notary-window.timer
```

The publisher never replaces a published month, and it stages each month in a hidden
directory, so the API never sees a partial one. It exits non-zero if the schedule has no
directory for the current month, and warns when 12 or fewer months remain; both show in
`systemctl status gkapi-notary-window` and the journal. No gkapi restart is needed; the
files are read per request.

**Order matters on first rollout:** deploy the binary from #192 first and let it run on the
flat files, and only then enable the timer. A binary that predates the schedule never reads
the month directories, and rolling back to one after a month is live breaks in-flight
donations (see Rollback).
