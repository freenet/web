"""Rewrite hardcoded absolute asset paths in Hugo HTML output for Freenet hosting.

Usage: python3 rewrite-paths.py <output-dir> <base-path>

Hugo templates use relURL which respects baseURL, but raw HTML in markdown
content (e.g., <img src="/img/foo.webp">) uses absolute paths that must be
rewritten to include the Freenet contract base path.
"""

import os
import re
import sys

output_dir = sys.argv[1]
base = sys.argv[2]

# Directories that contain assets referenced with absolute paths in content
ASSET_DIRS = r"(img|images|pdf|css|js)"

# Patterns: src="/img/...", srcset="/images/...", href="/pdf/..." (quoted and unquoted)
QUOTED = re.compile(r'((?:src|srcset|href)=")/' + ASSET_DIRS + r"/")
UNQUOTED = re.compile(r"((?:src|srcset|href)=)/" + ASSET_DIRS + r"/")

count = 0
for root, dirs, files in os.walk(output_dir):
    for fname in files:
        if not fname.endswith(".html"):
            continue
        path = os.path.join(root, fname)
        with open(path) as f:
            content = f.read()
        new_content = QUOTED.sub(rf"\1{base}/\2/", content)
        new_content = UNQUOTED.sub(rf"\1{base}/\2/", new_content)
        if new_content != content:
            with open(path, "w") as f:
                f.write(new_content)
            count += 1

print(f"Rewrote paths in {count} HTML files")
