"""Rewrite absolute paths in Hugo HTML output for Freenet hosting.

Usage: python3 rewrite-paths.py <output-dir> <base-path>

Hugo templates use relURL which respects baseURL, but raw HTML in markdown
content uses absolute paths like href="/about/news/" or src="/img/foo.webp"
that must be rewritten to include the Freenet contract base path.

This script rewrites ALL href/src/srcset attributes that start with "/"
(but not "//" protocol-relative or external URLs) to prepend the base path.
Paths that already start with the base path are left unchanged.
"""

import os
import re
import sys

output_dir = sys.argv[1]
base = sys.argv[2].rstrip("/")
base_stripped = base.lstrip("/")

count = 0
for root, dirs, files in os.walk(output_dir):
    for fname in files:
        if not fname.endswith(".html"):
            continue
        path = os.path.join(root, fname)
        with open(path) as f:
            content = f.read()

        def rewrite_quoted(m):
            prefix = m.group(1)  # e.g. href="
            path_after_slash = m.group(2)
            if path_after_slash.startswith(base_stripped):
                return m.group(0)
            return f'{prefix}{base}/{path_after_slash}'

        def rewrite_unquoted(m):
            prefix = m.group(1)  # e.g. href=
            path_after_slash = m.group(2)
            if path_after_slash.startswith(base_stripped):
                return m.group(0)
            return f'{prefix}{base}/{path_after_slash}'

        # Quoted: href="/foo" -> href="/base/foo" (but not href="//...")
        new_content = re.sub(
            r'((?:href|src|srcset)=")/((?!/)[^"]*)',
            rewrite_quoted, content
        )
        # Unquoted: href=/foo -> href=/base/foo
        new_content = re.sub(
            r'((?:href|src|srcset)=)/((?!/|"|>)\S*)',
            rewrite_unquoted, new_content
        )
        # Special case: href=/ (bare root, unquoted) -> href=/base/
        new_content = re.sub(
            r'(href=)/([>\s])',
            rf'\1{base}/\2', new_content
        )
        # Special case: href="/" (bare root, quoted)
        new_content = new_content.replace('href="/"', f'href="{base}/"')

        # Remove menu links to pages stripped from the Freenet build.
        # Match by href containing /ghostkey/ since the class name varies
        # with Hugo minification.
        new_content = re.sub(
            r'<a[^>]*href=["\']?[^"\'>\s]*/ghostkey/[^>]*>[^<]*</a>',
            '', new_content
        )

        if new_content != content:
            with open(path, "w") as f:
                f.write(new_content)
            count += 1

print(f"Rewrote paths in {count} HTML files")
