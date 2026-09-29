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

## Notary keys and the monthly schedule

The notary directory (`--notary-dir`, `/home/gkapi/delegate-keys` on nova) holds one
keypair per donation tier: `notary_certificate_<amount>.pem` and
`notary_signing_key_<amount>.pem` (older deployments use `delegate_*` names, which are
still read). The certificate's info string carries the amount and a date,
`delegate-key-created`, which every ghost key issued from it shows publicly.

To keep that date current without taking the master key out of storage every month, the
directory can also hold a schedule: one subdirectory per month, `YYYY-MM/`, each a complete
set of tiers. Generate it with the master key mounted, in one session:

```bash
rust/cli/generate_notary_keys.sh --master-key <master_signing_key.pem> \
  --notary-dir <output-dir> --start-month 2026-10 --months 120
```

By default $1 and $5 are dated monthly and $20 and up are dated yearly, because the date
partitions each tier's anonymity set and the higher tiers see too few donors a month for a
monthly date to be safe (see the script header). The script verifies every pair against the
compiled-in Freenet master key before writing it.

How the API uses it:

- **Quoting** (`/create-donation`, `/update-donation`) uses the newest `YYYY-MM/` that is not
  in the future, or the flat files if there is none, and records which on the PaymentIntent
  as `notary_period` metadata. If the current month is missing it logs an error and keeps
  issuing from the newest earlier month, so a schedule that runs out degrades to a stale
  date, not an outage. If the chosen month lacks a tier, that tier fails.
- **Signing** (`/sign-certificate`) uses exactly the pair the PaymentIntent was quoted from,
  never whatever is current by then. The browser blinds against the certificate it was
  quoted, before the card is charged, so signing with any other pair would charge the donor
  for a key that does not verify. A PaymentIntent with no `notary_period` (quoted from the
  flat files, including everything before the schedule existed) signs with the flat files.

Operationally, that means:

- **Do not delete a month directory, or the flat files, while a donation quoted from it
  could still be in checkout.** Removing one only breaks those in-flight donations, but it
  breaks them after the charge. Keep past months; they are small.
- Installing a schedule is a copy into the notary directory
  (`sudo cp -a <output-dir>/20?? /home/gkapi/delegate-keys/`, then
  `sudo chown -R gkapi:gkapi` and keep modes `700`/`600`). No restart is needed; the files
  are read per request.
- Keep the generated schedule on the encrypted master-key drive as well. It is the backup.
