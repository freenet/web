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
  const windowListeners = {};
  const window = {
    location: { hash },
    addEventListener: (ev, fn) => (windowListeners[ev] = fn),
  };
  vm.runInNewContext(match[1], { document, window });
  listeners.DOMContentLoaded();
  const shown = () =>
    ["open-link-missing", "open-link-invalid", "open-link-valid"].find(
      (id) => el(id).style.display === "",
    );
  const navigate = (newHash) => {
    window.location.hash = newHash;
    windowListeners.hashchange();
    return shown();
  };
  return { shown: shown(), el, navigate };
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
    const tryIt = el("open-link-try").href;
    const scheme = el("open-link-scheme").href;
    const want = "http://127.0.0.1:7509" + v.local_path;
    const wantTry = "https://try.freenet.org" + v.local_path;
    const rest = v.local_path.slice("/v1/contract/web/".length);
    if (local !== want) problem = `local button ${local} != ${want}`;
    else if (tryIt !== wantTry) problem = `try button ${tryIt} != ${wantTry}`;
    else if (scheme !== "freenet:" + rest)
      problem = `scheme button ${scheme} != freenet:${rest}`;
  }
  if (problem) {
    failures++;
    console.error(`FAIL ${JSON.stringify(v.raw.slice(0, 80))} (${v.note}): ${problem}`);
  }
}

// Local-only apps (the Ghost Key vault): no try.freenet.org button, a note
// saying why instead, and the local button unaffected. Driven through a
// hashchange to an ordinary id too, since both states must be reset.
const VAULT = "DLog47hEsrtuGT4N5XCeMBG45m4n1aWM89tBZXue2E1N";
const OTHER = "6FzSeAUKcqJrveKyU8RJgGKc5jRB1Z2juvxXtwTA4Em9";
{
  const check = (cond, what) => {
    if (!cond) {
      failures++;
      console.error(`FAIL local-only: ${what}`);
    }
  };
  const tryHidden = (el) => el("open-link-try-option").style.display === "none";
  const noteShown = (el) =>
    el("open-link-local-only").style.display === "" &&
    el("open-link-local-only").textContent.length > 0;

  const { shown, el, navigate } = runPage("#" + VAULT + "/");
  check(shown === "open-link-valid", `vault link showed ${shown}`);
  check(tryHidden(el), "try option visible for the vault");
  check(noteShown(el), "no local-only note for the vault");
  check(
    el("open-link-local").href === `http://127.0.0.1:7509/v1/contract/web/${VAULT}/`,
    `vault local button ${el("open-link-local").href}`,
  );

  check(navigate("#" + OTHER + "/") === "open-link-valid", "other id not valid");
  check(!tryHidden(el), "try option still hidden after leaving the vault");
  check(el("open-link-local-only").style.display === "none", "note still shown after leaving the vault");

  const fresh = runPage("#" + OTHER + "/");
  check(!tryHidden(fresh.el), "try option hidden for an ordinary id");
}

console.log(`${vectors.length} vectors, ${failures} failures`);
if (vectors.length < 40) {
  console.error("vector file looks truncated");
  process.exit(1);
}
process.exit(failures ? 1 : 0);
