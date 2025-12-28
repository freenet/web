# Notes on API setup

WARNING: This file is publicly readable, do NOT put anything secret in here.

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
