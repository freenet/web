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

# 3. back up with mv (keeps the capability on the backup, so rollback is clean)
ssh vega 'sudo mv /home/gkapi/bin/ghostkey-api \
                  /home/gkapi/bin/ghostkey-api.rollback-$(date +%Y%m%d-%H%M%S)'

# 4. install, own, and RESTORE THE CAPABILITY before restarting
ssh vega '
  sudo cp /tmp/ghostkey-api.new /home/gkapi/bin/ghostkey-api
  sudo chown gkapi:gkapi /home/gkapi/bin/ghostkey-api
  sudo chmod 775 /home/gkapi/bin/ghostkey-api
  sudo setcap cap_net_bind_service+ep /home/gkapi/bin/ghostkey-api
  getcap /home/gkapi/bin/ghostkey-api          # must print cap_net_bind_service=ep
'

# 5. only then restart
ssh vega 'sudo systemctl reset-failed gkapi && sudo systemctl restart gkapi'
```

Verify `getcap` prints the capability **before** restarting. If it is empty, fix it rather
than restarting to see what happens.

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
`hugo-site/themes/freenet/layouts/shortcodes/stripe-donation-form.html`):

```bash
for a in 1 5 20 50 100 500 2500 10000; do
  curl -s -X POST https://gkapi.freenet.org/create-donation \
    -H 'Content-Type: application/json' -d "{\"amount\":$((a*100)),\"currency\":\"usd\"}" \
  | grep -q notary_certificate_base64 && echo "\$$a ok" || echo "\$$a FAIL"
done
```

### Rollback

```bash
ssh vega '
  sudo cp /home/gkapi/bin/ghostkey-api.rollback-<stamp> /home/gkapi/bin/ghostkey-api
  sudo chown gkapi:gkapi /home/gkapi/bin/ghostkey-api
  sudo setcap cap_net_bind_service+ep /home/gkapi/bin/ghostkey-api
  sudo systemctl reset-failed gkapi && sudo systemctl restart gkapi
'
```

The `setcap` line is required on the way back too, for the reason above.

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
