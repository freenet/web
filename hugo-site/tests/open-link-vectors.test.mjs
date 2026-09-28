// Runs the /open page's own validation code against the share-link vectors
// that freenet-core's freenet:// handler is also tested against
// (crates/core/tests/data/share-link-vectors.json there; a copy here, kept
// identical by the drift check in .github/workflows/open-link.yml). A rule
// changed on one side only fails a test.
//
// Usage: node hugo-site/tests/open-link-vectors.test.mjs

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const here = dirname(fileURLToPath(import.meta.url));
const shortcode = readFileSync(
  join(here, "../themes/freenet/layouts/shortcodes/open-link.html"),
  "utf8",
);
const match = shortcode.match(/<script>([\s\S]*?)<\/script>/);
if (!match) throw new Error("no <script> block in open-link.html");

// Run the page script with a stub DOM, then drive processFragment() the way
// the browser does: set location.hash, fire DOMContentLoaded, read the state
// and the button hrefs it produced.
function runPage(hash) {
  const elements = {};
  const el = (id) =>
    (elements[id] ??= { id, style: {}, textContent: "", href: "#" });
  const listeners = {};
  const document = {
    getElementById: el,
    addEventListener: (ev, fn) => (listeners[ev] = fn),
  };
  const window = {
    location: { hash },
    addEventListener: () => {},
  };
  vm.runInNewContext(match[1], { document, window });
  listeners.DOMContentLoaded();
  const shown = ["open-link-missing", "open-link-invalid", "open-link-valid"].find(
    (id) => el(id).style.display === "",
  );
  return { shown, el };
}

const { vectors } = JSON.parse(
  readFileSync(join(here, "share-link-vectors.json"), "utf8"),
);
let failures = 0;
for (const v of vectors) {
  const { shown, el } = runPage("#" + v.raw);
  const valid = shown === "open-link-valid";
  let problem = null;
  if (valid !== v.valid) {
    problem = `expected valid=${v.valid}, page showed ${shown}`;
  } else if (valid) {
    const local = el("open-link-local").href;
    const scheme = el("open-link-scheme").href;
    const want = "http://127.0.0.1:7509" + v.local_path;
    const rest = v.local_path.slice("/v1/contract/web/".length);
    if (local !== want) problem = `local button ${local} != ${want}`;
    else if (scheme !== "freenet:" + rest)
      problem = `scheme button ${scheme} != freenet:${rest}`;
  }
  if (problem) {
    failures++;
    console.error(`FAIL ${JSON.stringify(v.raw.slice(0, 80))} (${v.note}): ${problem}`);
  }
}
console.log(`${vectors.length} vectors, ${failures} failures`);
if (vectors.length < 40) {
  console.error("vector file looks truncated");
  process.exit(1);
}
process.exit(failures ? 1 : 0);
