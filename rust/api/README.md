# Notes on API setup

WARNING: This file is publicly readable, do NOT put anything secret in here.

## Deploying gkapi

There is **no CI deployment for this crate**. `deploy.yml` builds the Hugo site and
publishes it to GitHub Pages; it never touches the API. `rust-api-tests.yml` only runs
fmt, build and test. Merging a change to `rust/api` therefore ships nothing: the binary
on vega has to be replaced by hand, and has in the past sat months behind main.

There is also no Rust toolchain on vega (`~gkapi/.cargo` exists but `bin/` is empty), so
the binary is built elsewhere and copied. vega and nova are both Ubuntu 24.04 on the same
glibc, so a release build from nova runs there as-is. Check with `ldd --version` on both
before relying on that.

### The step that is easy to miss

The service runs as the unprivileged `gkapi` user and binds **port 80** for the HTTP-01
ACME challenge server, which needs `CAP_NET_BIND_SERVICE`. That capability lives on the
binary as a file capability, and **`cp` does not preserve it**. Replace the binary without
re-running `setcap` and the service binds 443, logs `Listening on 0.0.0.0:443`, then panics
at `main.rs` with `PermissionDenied` on the port 80 bind and systemd restart-loops until it
gives up.

The same trap catches the rollback: copying a backup *back* into place also produces a file
with no capability, so a panicking service stays panicking and it looks like the new build
is at fault. `mv` preserves the capability because it keeps the inode; `cp` does not.

### Procedure

```bash
# 1. build on nova, from main, and prove the build contains what you expect
cd ~/code/freenet/web/main/rust && cargo build --release -p ghostkey-api
strings target/release/ghostkey-api | grep -c payment_claim   # sanity: expect non-zero

# 2. upload and re-check after transfer
scp target/release/ghostkey-api vega:/tmp/ghostkey-api.new
ssh vega 'strings /tmp/ghostkey-api.new | grep -q payment_claim && echo ok'

# 3. stage it at its final location, fully prepared, WITHOUT displacing the live binary
ssh vega '
  sudo cp /tmp/ghostkey-api.new /home/gkapi/bin/ghostkey-api.staged
  sudo chown gkapi:gkapi        /home/gkapi/bin/ghostkey-api.staged
  sudo chmod 775                /home/gkapi/bin/ghostkey-api.staged
  sudo setcap cap_net_bind_service+ep /home/gkapi/bin/ghostkey-api.staged
  sudo getcap /home/gkapi/bin/ghostkey-api.staged
'
# Must print: /home/gkapi/bin/ghostkey-api.staged cap_net_bind_service=ep
# If it does not, STOP. Nothing has changed yet and the live binary is untouched.

# 4. swap with two adjacent renames, then restart
ssh vega '
  set -e
  STAMP=$(date +%Y%m%d-%H%M%S)
  sudo mv /home/gkapi/bin/ghostkey-api         /home/gkapi/bin/ghostkey-api.rollback-$STAMP
  sudo mv /home/gkapi/bin/ghostkey-api.staged  /home/gkapi/bin/ghostkey-api
  echo "rollback binary: /home/gkapi/bin/ghostkey-api.rollback-$STAMP"
  sudo systemctl reset-failed gkapi
  sudo systemctl restart gkapi
'
```

Two things about that shape are deliberate, and both exist to avoid a worse failure than
the one being fixed:

- **Prepare the capability on the staged file, and verify it, before anything is
  displaced.** The `getcap` gate in step 3 is the whole point of splitting the steps: if it
  fails you stop with the running service completely untouched. Do not restart to see what
  happens.
- **Do not `mv` the old binary away in one command and `cp` the new one in with another.**
  Between those two the path does not exist, and anything that restarts the unit in that
  window (a crash, an OOM, a reboot, someone running `systemctl restart` out of order) fails
  with `status=203/EXEC` and stays down. Step 4 closes that to two adjacent renames.
  `mv` is also what carries the capability across, since it keeps the inode.

`sudo` on `getcap` is not decoration: it lives in `/usr/sbin`, which is not on a normal
user's `PATH`, so a bare `getcap` can fail as "command not found" and skip the check
entirely.

### Verifying a deploy

```bash
ssh vega 'systemctl is-active gkapi; strings /home/gkapi/bin/ghostkey-api | grep -c payment_claim'
curl -s https://gkapi.freenet.org/                       # {"message":"Hello, world!"}
curl -s -o /dev/null -w '%{http_code}\n' http://gkapi.freenet.org/.well-known/acme-challenge/probe
```

That last one matters: a 404 means the port 80 challenge listener is up. Connection refused
means the capability is missing and certificate renewal will fail at the next attempt even
if HTTPS looks healthy.

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

Substitute `<stamp>` with the timestamp step 4 printed.

```bash
ssh vega '
  set -e
  sudo cp /home/gkapi/bin/ghostkey-api.rollback-<stamp> /home/gkapi/bin/ghostkey-api.staged
  sudo chown gkapi:gkapi /home/gkapi/bin/ghostkey-api.staged
  sudo chmod 775 /home/gkapi/bin/ghostkey-api.staged
  sudo setcap cap_net_bind_service+ep /home/gkapi/bin/ghostkey-api.staged
  sudo getcap /home/gkapi/bin/ghostkey-api.staged
  sudo mv /home/gkapi/bin/ghostkey-api.staged /home/gkapi/bin/ghostkey-api
  sudo systemctl reset-failed gkapi && sudo systemctl restart gkapi
'
```

The `setcap` line is required on the way back too, and this is the part that is genuinely
easy to get wrong under pressure: copying a backup *into place* produces a file with no
capability, so the service carries on panicking and it reads as "the new build is broken"
rather than "the copy dropped a capability". If a rollback does not fix the panic, run
`sudo getcap` on the live binary before concluding anything about the build.

## letsencrypt

Verify that certificate was automatically renewed by root cron job on vega by looking at write times
of cert files:

```
root@vega:/home/gkapi# cd /etc/letsencrypt/live/gkapi.freenet.org/
root@vega:/etc/letsencrypt/live/gkapi.freenet.org# ls -l
total 4
-rw-r--r-- 1 root ssl-cert 692 Aug  5 00:40 README
lrwxrwxrwx 1 root root      41 Oct  4 05:31 cert.pem -> ../../archive/gkapi.freenet.org/cert2.pem
lrwxrwxrwx 1 root root      42 Oct  4 05:31 chain.pem -> ../../archive/gkapi.freenet.org/chain2.pem
lrwxrwxrwx 1 root root      46 Oct  4 05:31 fullchain.pem -> ../../archive/gkapi.freenet.org/fullchain2.pem
lrwxrwxrwx 1 root root      44 Oct  4 05:31 privkey.pem -> ../../archive/gkapi.freenet.org/privkey2.pem
```
