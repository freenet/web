#!/usr/bin/env python3
"""Fail the build if the generated site contains a broken internal link.

Usage: python3 scripts/check-links.py [output-dir]
       python3 scripts/check-links.py --self-test

Checks every same-site reference in the built HTML for two failure modes:

* the target page or file does not exist in the output, and
* the target page exists but has no element with the ``#fragment`` id.

The second one is what motivated this script: a FAQ entry was deleted in March
2025 but its table-of-contents link was left behind, so
``/about/faq/#what-is-the-status-of-freenet`` pointed at nothing for a year
before a user reported it. Hugo does not verify fragments, and neither did CI.

Links to other sites are left alone so the check stays deterministic and
offline. The site's own hostname is read from the generated sitemap, so a build
with a non-default ``baseURL`` is still checked rather than silently skipped.
"""

import os
import re
import sys
import urllib.parse
from html.parser import HTMLParser

# The production hostnames. Content links to these absolutely, so they count as
# internal whatever baseURL a particular build used; whatever host the build
# itself emitted is added to this set at run time (see site_hosts).
PRODUCTION_HOSTS = {"freenet.org", "www.freenet.org"}

# Attributes that name something which has to exist in the output. Anchors are
# the reason this script exists, but a missing image or stylesheet is just as
# user-visible and just as invisible to Hugo.
REFERENCE_ATTRS = {
    "a": ("href",),
    "area": ("href",),
    "audio": ("src",),
    "embed": ("src",),
    "iframe": ("src",),
    "img": ("src", "srcset"),
    "link": ("href",),
    "object": ("data",),
    "script": ("src",),
    "source": ("src", "srcset"),
    "track": ("src",),
    "video": ("poster", "src"),
}

# "top" always scrolls to the top of the document, with or without a matching
# element. https://html.spec.whatwg.org/multipage/browsing-the-web.html#scrolling-to-a-fragment
ALWAYS_VALID_FRAGMENTS = {"top"}

MAX_REDIRECTS = 5

# <meta http-equiv=refresh content="0; url=..."> — only meaningful in <head>,
# which is where Hugo puts it for an alias page.
REFRESH_URL_RE = re.compile(r"""url\s*=\s*['"]?([^'";\s]+)""", re.IGNORECASE)

SITEMAP_LOC_RE = re.compile(r"<loc>\s*([^<\s]+)\s*</loc>", re.IGNORECASE)


def first_attr(attrs):
    """Fold a parsed attribute list into a dict, keeping the FIRST of any
    duplicate, which is the one a browser honours."""
    folded = {}
    for name, value in attrs:
        name = name.lower()
        if name not in folded:
            folded[name] = value
    return folded


class PageParser(HTMLParser):
    """Collect the anchor ids a page defines and the references it makes."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.ids = set()
        self.references = []
        self.redirect_to = None
        self._in_head = False

    def handle_starttag(self, tag, attrs):
        tag = tag.lower()
        attrs = first_attr(attrs)

        if tag == "head":
            self._in_head = True
        elif tag == "body":
            self._in_head = False

        if attrs.get("id"):
            self.ids.add(attrs["id"])
        # Pre-HTML5 named anchors are still valid fragment targets.
        if tag == "a" and attrs.get("name"):
            self.ids.add(attrs["name"])

        # Hugo writes alias pages as a meta-refresh stub. Only honour one in
        # <head>: a manual page may legitimately *document* a refresh tag, and
        # treating that page as a redirect would send fragment checks
        # somewhere else entirely.
        if (
            tag == "meta"
            and self._in_head
            and self.redirect_to is None
            and (attrs.get("http-equiv") or "").strip().lower() == "refresh"
        ):
            match = REFRESH_URL_RE.search(attrs.get("content") or "")
            if match:
                self.redirect_to = match.group(1)

        for attr in REFERENCE_ATTRS.get(tag, ()):
            value = attrs.get(attr)
            if not value:
                continue
            if attr == "srcset":
                # "a.png 1x, b.png 2x" -> each candidate URL
                for candidate in value.split(","):
                    candidate = candidate.strip().split()
                    if candidate:
                        self.references.append(candidate[0])
            else:
                self.references.append(value)

    def handle_endtag(self, tag):
        if tag.lower() == "head":
            self._in_head = False


def load_pages(root):
    """Map each ``/path/to/page.html`` in the output to its parsed contents."""
    pages = {}
    for dirpath, _dirnames, filenames in os.walk(root):
        for name in filenames:
            if not name.endswith(".html"):
                continue
            path = os.path.join(dirpath, name)
            with open(path, encoding="utf-8", errors="replace") as handle:
                source = handle.read()
            parser = PageParser()
            parser.feed(source)
            parser.close()
            key = "/" + os.path.relpath(path, root).replace(os.sep, "/")
            pages[key] = parser
    return pages


def site_hosts(root):
    """The hostnames that mean "this site".

    Hugo stamps the configured baseURL into sitemap.xml, so reading it back
    keeps the check honest for a build published under a different host
    instead of silently treating its own links as external.
    """
    hosts = set(PRODUCTION_HOSTS)
    sitemap = os.path.join(root, "sitemap.xml")
    if os.path.isfile(sitemap):
        with open(sitemap, encoding="utf-8", errors="replace") as handle:
            match = SITEMAP_LOC_RE.search(handle.read())
        if match:
            hostname = urllib.parse.urlparse(match.group(1)).hostname
            if hostname:
                hosts.add(hostname.lower())
    return hosts


def page_key(pages, path):
    """Resolve a URL path to the HTML file that serves it, if any."""
    if path.endswith("/"):
        candidate = path + "index.html"
    elif path.endswith(".html"):
        candidate = path
    else:
        candidate = path + "/index.html"
    return candidate if candidate in pages else None


def follow_redirects(pages, key):
    """Chase meta-refresh alias stubs to the page that actually has content.

    Returns ``(key, None)`` on success or ``(None, reason)`` if the chain does
    not land anywhere real.
    """
    seen = set()
    while True:
        page = pages[key]
        if not page.redirect_to:
            return key, None
        if key in seen:
            return None, "redirect loop"
        if len(seen) >= MAX_REDIRECTS:
            return None, "redirect chain longer than %d hops" % MAX_REDIRECTS
        seen.add(key)
        target = urllib.parse.urlparse(
            urllib.parse.urljoin("https://site.invalid" + key, page.redirect_to)
        )
        next_key = page_key(pages, target.path)
        if next_key is None:
            return None, "redirect target does not exist"
        key = next_key


def check(root):
    """Return ``(page_count, broken)`` where broken is a list of
    ``(page, reference, reason)``."""
    pages = load_pages(root)
    hosts = site_hosts(root)
    broken = []
    for source_key, page in sorted(pages.items()):
        for reference in page.references:
            url = urllib.parse.urlparse(
                urllib.parse.urljoin("https://" + sorted(hosts)[0] + source_key, reference)
            )
            if url.scheme not in ("http", "https"):
                continue  # mailto:, data:, javascript:, ...
            if (url.hostname or "").lower() not in hosts:
                continue  # another site; not ours to verify
            fragment = urllib.parse.unquote(url.fragment)
            path = urllib.parse.unquote(url.path)

            target_key = page_key(pages, url.path)
            if target_key is None:
                # Not a page, so it has to be a file that exists — a directory
                # with no index.html is a 404, not a valid target.
                if not os.path.isfile(os.path.join(root, path.lstrip("/"))):
                    broken.append((source_key, reference, "target does not exist"))
                continue

            if not fragment or fragment in ALWAYS_VALID_FRAGMENTS:
                continue
            # Check the page's own ids before following any redirect: a real
            # page can carry a meta refresh and still be the fragment's home.
            if fragment in pages[target_key].ids:
                continue
            resolved, reason = follow_redirects(pages, target_key)
            if resolved is None:
                broken.append((source_key, reference, reason))
            elif fragment not in pages[resolved].ids:
                broken.append((source_key, reference, "no #%s on the target page" % fragment))
    return len(pages), broken


def report(root):
    total, broken = check(root)
    if total == 0:
        print("No HTML pages found in %s (build the site first)" % root, file=sys.stderr)
        return 2
    for source_key, reference, reason in broken:
        print("%s -> %s: %s" % (source_key, reference, reason), file=sys.stderr)
    if broken:
        print(
            "\n%d broken internal link(s) across %d page(s)." % (len(broken), total),
            file=sys.stderr,
        )
        return 1
    print("No broken internal links (%d pages checked)." % total)
    return 0


def self_test():
    """Prove the checker actually fails on the bugs it claims to catch.

    A link checker that silently stops checking is worse than none at all, so
    CI runs this before trusting a clean report on the real site. Every case
    below is one a previous version of this script got wrong.
    """
    import shutil
    import tempfile

    root = tempfile.mkdtemp(prefix="check-links-selftest-")
    try:

        def write(path, body):
            full = os.path.join(root, path)
            os.makedirs(os.path.dirname(full), exist_ok=True)
            with open(full, "w", encoding="utf-8") as handle:
                handle.write(body)

        write(
            "faq/index.html",
            """
            <h1 id="what-is-freenet">What is Freenet?</h1>
            <a href="#what-is-freenet">good fragment</a>
            <a href="#deleted-section">BAD fragment</a>
            <a name="legacy-anchor"></a>
            <a href="#legacy-anchor">good named anchor</a>
            <a href="#top">good: #top always resolves</a>
            <a href="/about/">good page</a>
            <a href="/nope/">BAD page</a>
            <a href="/paper.pdf">good static file</a>
            <a href="/missing.pdf">BAD static file</a>
            <a href="/assets/">BAD: directory with no index.html</a>
            <img src="/logo.png">
            <img src="/missing.png">
            <img srcset="/logo.png 1x, /missing-2x.png 2x">
            <link rel="stylesheet" href="/site.css">
            <script src="/missing.js"></script>
            <a href="https://example.com/#whatever">external, not checked</a>
            <a href="mailto:x@example.com">mailto, not checked</a>
            <a href="/old-faq/#what-is-freenet">good through alias</a>
            <a href="/old-faq/#gone">BAD through alias</a>
            <a href="/loop-a/#anything">BAD: redirect loop</a>
            <a href="/documented/#really-here">good: refresh in body is not an alias</a>
            <a href="/body-refresh/#what-is-freenet">BAD: body refresh is not an alias to follow</a>
            <a href="/dup-attr/" href="/about/">BAD: a browser honours the first href</a>
            <a href="HTTPS://FREENET.ORG/nope/">BAD: uppercase host is still us</a>
            <a href="https://freenet.org:443/nope/">BAD: default port is still us</a>
            """,
        )
        write("about/index.html", "<p>hi</p>")
        write("old-faq/index.html", '<head><meta http-equiv="refresh" content="0; url=/faq/"></head>')
        write("loop-a/index.html", '<head><meta http-equiv="refresh" content="0; url=/loop-b/"></head>')
        write("loop-b/index.html", '<head><meta http-equiv="refresh" content="0; url=/loop-a/"></head>')
        # A real page that merely *documents* a meta refresh must not be
        # mistaken for an alias stub.
        write(
            "documented/index.html",
            "<head><title>t</title></head><body><h2 id=really-here>x</h2>"
            "<pre>&lt;meta http-equiv=\"refresh\" content=\"0; url=/faq/\"&gt;</pre></body>",
        )
        # A refresh tag outside <head> does not make the page an alias, so a
        # fragment must be found on the page itself, not on the refresh target.
        write(
            "body-refresh/index.html",
            '<head><title>t</title></head><body><meta http-equiv="refresh" '
            'content="0; url=/faq/"><p>real content</p></body>',
        )
        write("paper.pdf", "%PDF-1.4")
        write("logo.png", "PNG")
        write("site.css", "body{}")
        write("assets/keep.txt", "not an index")

        _total, broken = check(root)
        found = sorted(reference for _src, reference, _reason in broken)
        expected = sorted(
            [
                "#deleted-section",
                "/nope/",
                "/missing.pdf",
                "/assets/",
                "/missing.png",
                "/missing-2x.png",
                "/missing.js",
                "/old-faq/#gone",
                "/loop-a/#anything",
                "/body-refresh/#what-is-freenet",
                "/dup-attr/",
                "HTTPS://FREENET.ORG/nope/",
                "https://freenet.org:443/nope/",
            ]
        )
        if found != expected:
            print("self-test FAILED", file=sys.stderr)
            print("  missing: %s" % sorted(set(expected) - set(found)), file=sys.stderr)
            print("  unexpected: %s" % sorted(set(found) - set(expected)), file=sys.stderr)
            return 1
        print("self-test passed (%d expected failures detected)" % len(expected))
        return 0
    finally:
        shutil.rmtree(root, ignore_errors=True)


def main(argv):
    if "--self-test" in argv:
        return self_test()
    root = argv[1] if len(argv) > 1 else "hugo-site/public"
    if not os.path.isdir(root):
        print("No such directory: %s (build the site first)" % root, file=sys.stderr)
        return 2
    return report(root)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
