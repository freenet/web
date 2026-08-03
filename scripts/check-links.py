#!/usr/bin/env python3
"""Fail the build if the generated site contains a broken internal link.

Usage: python3 scripts/check-links.py [output-dir]
       python3 scripts/check-links.py --self-test

Checks every ``<a href>`` in the built HTML for two failure modes:

* the target page or file does not exist in the output, and
* the target page exists but has no element with the ``#fragment`` id.

The second one is what motivated this script: a FAQ entry was deleted in March
2025 but its table-of-contents link was left behind, so
``/about/faq/#what-is-the-status-of-freenet`` pointed at nothing for a year
before a user reported it. Hugo does not verify fragments, and neither did CI.

Only same-site links are checked. External URLs are left alone so the check
stays deterministic and offline.
"""

import os
import re
import sys
import urllib.parse
from html.parser import HTMLParser

# Links whose target lives outside the built output. Everything else that
# resolves to this site must exist.
INTERNAL_HOSTS = {"", "freenet.org", "www.freenet.org"}

# Hugo writes alias pages as a stub with a meta refresh; follow those so a
# fragment link through an alias is checked against the real page.
REFRESH_RE = re.compile(
    r"""<meta[^>]+http-equiv=["']?refresh["']?[^>]*""" r"""content=["'][^"']*?url=([^"'\s]+)""",
    re.IGNORECASE,
)
MAX_REDIRECTS = 5


class PageParser(HTMLParser):
    """Collect the anchor ids a page defines and the links it makes."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.ids = set()
        self.hrefs = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get("id"):
            self.ids.add(attrs["id"])
        if tag == "a":
            # Pre-HTML5 named anchors are still valid fragment targets.
            if attrs.get("name"):
                self.ids.add(attrs["name"])
            if attrs.get("href") is not None:
                self.hrefs.append(attrs["href"])


class Page:
    def __init__(self, ids, hrefs, redirect_to):
        self.ids = ids
        self.hrefs = hrefs
        self.redirect_to = redirect_to


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
            match = REFRESH_RE.search(source)
            redirect_to = match.group(1) if match else None
            key = "/" + os.path.relpath(path, root).replace(os.sep, "/")
            pages[key] = Page(parser.ids, parser.hrefs, redirect_to)
    return pages


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
    """Chase meta-refresh alias stubs to the page that actually has content."""
    seen = set()
    for _ in range(MAX_REDIRECTS):
        page = pages[key]
        if not page.redirect_to or key in seen:
            return key
        seen.add(key)
        target = urllib.parse.urlparse(urllib.parse.urljoin("https://freenet.org" + key, page.redirect_to))
        if target.netloc not in INTERNAL_HOSTS:
            return None
        next_key = page_key(pages, target.path)
        if next_key is None:
            return None
        key = next_key
    return key


def check(root):
    """Return a list of ``(page, href, reason)`` for every broken link found."""
    pages = load_pages(root)
    broken = []
    for source_key, page in sorted(pages.items()):
        for href in page.hrefs:
            url = urllib.parse.urlparse(
                urllib.parse.urljoin("https://freenet.org" + source_key, href)
            )
            if url.scheme not in ("http", "https") or url.netloc not in INTERNAL_HOSTS:
                continue  # mailto:, external site, protocol-relative CDN, ...
            fragment = urllib.parse.unquote(url.fragment)
            target_key = page_key(pages, url.path)
            if target_key is None:
                # Not a page; it may still be a static file such as a PDF.
                on_disk = os.path.join(root, urllib.parse.unquote(url.path).lstrip("/"))
                if not os.path.exists(on_disk):
                    broken.append((source_key, href, "target does not exist"))
                continue
            if not fragment:
                continue
            resolved = follow_redirects(pages, target_key)
            if resolved is None:
                broken.append((source_key, href, "redirect target does not exist"))
            elif fragment not in pages[resolved].ids:
                broken.append((source_key, href, "no #%s on the target page" % fragment))
    return len(pages), broken


def report(root):
    total, broken = check(root)
    for source_key, href, reason in broken:
        print("%s -> %s: %s" % (source_key, href, reason), file=sys.stderr)
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
    CI runs this before trusting a clean report on the real site.
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
            <a href="/about/">good page</a>
            <a href="/nope/">BAD page</a>
            <a href="/paper.pdf">good static file</a>
            <a href="/missing.pdf">BAD static file</a>
            <a href="https://example.com/#whatever">external, not checked</a>
            <a href="mailto:x@example.com">mailto, not checked</a>
            <a href="/old-faq/#what-is-freenet">good through alias</a>
            <a href="/old-faq/#gone">BAD through alias</a>
            """,
        )
        write("about/index.html", "<p>hi</p>")
        write(
            "old-faq/index.html",
            '<meta http-equiv="refresh" content="0; url=/faq/">',
        )
        write("paper.pdf", "%PDF-1.4")

        _total, broken = check(root)
        found = sorted(href for _src, href, _reason in broken)
        expected = sorted(
            ["#deleted-section", "/nope/", "/missing.pdf", "/old-faq/#gone"]
        )
        if found != expected:
            print("self-test FAILED", file=sys.stderr)
            print("  expected: %s" % expected, file=sys.stderr)
            print("  found:    %s" % found, file=sys.stderr)
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
