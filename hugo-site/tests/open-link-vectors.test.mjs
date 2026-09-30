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
//
// The stub only needs to be complete enough that the REAL page script runs
// to completion without throwing -- it does not need to be a faithful DOM.
// The page has grown UI-layout code (layoutButtons(), the share-link maker
// tool's event wiring) that this test doesn't care about and never exercises
// meaningfully, but which the script still executes on every load. Each time
// the page starts calling a DOM method this stub doesn't have, add a no-op
// stand-in for it here rather than changing the page to avoid calling it --
// the assertions below (display state + button hrefs) are unaffected either
// way, since they never depend on classList/appendChild/querySelector output.
function runPage(hash) {
  const elements = {};
  function makeElement(id) {
    return {
      id,
      style: {},
      textContent: "",
      href: "#",
      value: "",
      checked: false,
      classList: { add() {}, remove() {}, contains: () => false },
      addEventListener() {},
      appendChild() {},
      querySelector: () => makeElement(`${id}::child`),
    };
  }
  const el = (id) => (elements[id] ??= makeElement(id));
  const listeners = {};
  const document = {
    getElementById: el,
    addEventListener: (ev, fn) => (listeners[ev] = fn),
    querySelector: () => makeElement("document::query"),
    querySelectorAll: () => [],
  };
  const window = {
    location: { hash, search: "" },
    addEventListener: () => {},
  };
  vm.runInNewContext(match[1], { document, window });
  listeners.DOMContentLoaded();
  const shown = ["open-link-missing", "open-link-invalid", "open-link-valid"].find(
    (id) => el(id).style.display === "",
  );
  return { shown, el };
}

const { vectors } = JSON.parse(readFileSync(join(here, "share-link-vectors.json"), "utf8"));
let failures = 0;
for (const v of vectors) {
  const { shown, el } = runPage("#" + v.raw);
  const valid = shown === "open-link-valid";
  let problem = null;
  if (valid !== v.valid) {
    problem = `expected valid=${v.valid}, page showed ${shown}`;
  } else if (valid) {
    const local = el("open-link-local").href;
    const tryIt = el("open-link-try").href;
    const scheme = el("open-link-scheme").href;
    const want = "http://127.0.0.1:7509" + v.local_path;
    const wantTry = "https://try.freenet.org" + v.local_path;
    const rest = v.local_path.slice("/v1/contract/web/".length);
    if (local !== want) problem = `local button ${local} != ${want}`;
    else if (tryIt !== wantTry) problem = `try button ${tryIt} != ${wantTry}`;
    else if (scheme !== "freenet:" + rest) problem = `scheme button ${scheme} != freenet:${rest}`;
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
