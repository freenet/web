// Runs the /open page's own validation code against the share-link vectors
// that freenet-core's freenet:// handler is also tested against
// (crates/core/tests/data/share-link-vectors.json there; a copy here, kept
// identical by the drift check in .github/workflows/open-link.yml). A rule
// changed on one side only fails a test.
//
// Also drives the two features built on that same validation: the
// `?via=browser` auto-redirect and the share-link maker tool. Both reuse the
// exact parseContractId/validateRest/splitFragment the base test exercises,
// so this is regression coverage for them, not a second independent check.
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

// Runs the page script with a stub DOM against one `location.hash` +
// `location.search`, drives it the way a browser would (fire
// DOMContentLoaded), and returns enough to check both the "open a link"
// behavior and the share-link maker tool.
//
// The stub only needs to be complete enough that the REAL page script runs
// to completion without throwing -- it does not need to be a faithful DOM.
// The page has grown UI-layout code (layoutButtons()) that this test doesn't
// care about and never exercises meaningfully, but which the script still
// executes on every load. Each time the page starts calling a DOM method
// this stub doesn't have, add a no-op stand-in for it here rather than
// changing the page to avoid calling it -- the assertions below (display
// state, button hrefs, redirect target, maker output) are unaffected either
// way, since none of them depend on classList/appendChild/querySelector
// output for anything other than the two "who is this for" radios, which
// the stub below backs with real shared, mutable state.
function runPage({ hash = "", search = "" } = {}) {
  const elements = {};
  function makeElement(id) {
    const elListeners = {};
    return {
      id,
      style: {},
      textContent: "",
      href: "#",
      value: "",
      checked: false,
      classList: { add() {}, remove() {}, contains: () => false },
      addEventListener(ev, fn) {
        (elListeners[ev] ??= []).push(fn);
      },
      _fire(ev) {
        (elListeners[ev] || []).forEach((fn) => fn({}));
      },
      appendChild() {},
      querySelector: () => makeElement(`${id}::child`),
    };
  }
  const el = (id) => (elements[id] ??= makeElement(id));

  // The two "who is this for" radios need to be real, shared, mutable
  // stand-ins -- not a fresh throwaway per lookup -- so that the maker
  // tool's own ":checked" read sees a write the page's own code just made
  // (e.g. a pasted try.freenet.org link's via hint checking the browser
  // radio). "local" starts checked, matching the real markup's `checked`
  // attribute.
  const viaLocalRadio = makeElement("open-maker-via-local");
  viaLocalRadio.value = "local";
  viaLocalRadio.checked = true;
  const viaBrowserRadio = makeElement("open-maker-via-browser");
  viaBrowserRadio.value = "browser";
  viaBrowserRadio.checked = false;

  const listeners = {};
  const document = {
    getElementById: el,
    addEventListener: (ev, fn) => (listeners[ev] = fn),
    querySelector: (sel) => {
      if (typeof sel === "string" && sel.indexOf("open-maker-via") !== -1) {
        if (sel.indexOf(":checked") !== -1) {
          return viaBrowserRadio.checked ? viaBrowserRadio : viaLocalRadio;
        }
        if (sel.indexOf('"browser"') !== -1) return viaBrowserRadio;
        if (sel.indexOf('"local"') !== -1) return viaLocalRadio;
      }
      return makeElement("document::query");
    },
    querySelectorAll: (sel) =>
      typeof sel === "string" && sel.indexOf("open-maker-via") !== -1
        ? [viaLocalRadio, viaBrowserRadio]
        : [],
  };
  let replacedTo = null;
  const windowListeners = {};
  const window = {
    location: { hash, search, replace: (url) => (replacedTo = url) },
    addEventListener: (ev, fn) => (windowListeners[ev] = fn),
  };
  vm.runInNewContext(match[1], { document, window, URLSearchParams });
  listeners.DOMContentLoaded();
  const currentState = () =>
    ["open-link-missing", "open-link-invalid", "open-link-valid"].find(
      (id) => el(id).style.display === "",
    );

  return {
    shown: currentState(),
    el,
    get replacedTo() {
      return replacedTo;
    },
    // Changes the fragment in place and fires hashchange, as editing the
    // address bar would. Returns the state shown afterwards.
    navigate(newHash) {
      window.location.hash = newHash;
      windowListeners.hashchange();
      return currentState();
    },
    // Drives the share-link maker tool exactly as a click would: fill the
    // input, fire the generate button's click listener, read back the
    // result. Only meaningful when `shown === "open-link-missing"`, i.e.
    // this `runPage` was called with an empty hash.
    runMaker(pastedText) {
      el("open-maker-input").value = pastedText;
      el("open-maker-generate")._fire("click");
      return {
        error:
          el("open-maker-error").style.display !== "none"
            ? el("open-maker-error").textContent
            : null,
        output: el("open-maker-output").value,
      };
    },
  };
}

const { vectors } = JSON.parse(readFileSync(join(here, "share-link-vectors.json"), "utf8"));
let failures = 0;

// --- The base page: open a link -------------------------------------------
for (const v of vectors) {
  const { shown, el } = runPage({ hash: "#" + v.raw });
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

// --- ?via=browser: must redirect exactly the valid vectors, and never the
// invalid ones, to the fixed try.freenet.org destination. Using real
// URLSearchParams here (passed into the vm context above) matters: without
// it, `getViaParam()`'s try/catch silently swallows the missing global and
// always returns null, which would make every assertion in this block pass
// vacuously without ever exercising the redirect decision at all. ---
let viaFailures = 0;
for (const v of vectors) {
  const { shown, replacedTo } = runPage({ hash: "#" + v.raw, search: "?via=browser" });
  if (v.valid) {
    const want = "https://try.freenet.org" + v.local_path;
    if (replacedTo !== want) {
      viaFailures++;
      console.error(
        `FAIL via=browser ${JSON.stringify(v.raw.slice(0, 80))} (${v.note}): redirected to ${replacedTo}, want ${want}`,
      );
    }
  } else if (replacedTo !== null) {
    // Not "must show open-link-invalid": an empty vector legitimately shows
    // open-link-missing instead (a different, also-correct non-valid
    // state -- see the base loop above, which already treats that as
    // valid=false). The one invariant that matters for `via=browser`
    // specifically is that nothing not valid ever redirects anywhere.
    viaFailures++;
    console.error(
      `FAIL via=browser ${JSON.stringify(v.raw.slice(0, 80))} (${v.note}): must never redirect ` +
        `(shown=${shown}, replacedTo=${replacedTo})`,
    );
  }
}
console.log(`${vectors.length} via=browser vectors, ${viaFailures} failures`);

// --- Share-link maker tool: every accepted input shape must accept exactly
// the vectors the main page accepts, and reject exactly the ones it
// rejects. ---
let makerFailures = 0;
const makerShapes = [
  (v) => v.raw,
  (v) => "freenet:" + v.raw,
  (v) => "freenet://" + v.raw,
  (v) => "http://127.0.0.1:7509/v1/contract/web/" + v.raw,
  (v) => "https://freenet.org/open#" + v.raw,
];
let makerCases = 0;
for (const v of vectors) {
  for (const shape of makerShapes) {
    makerCases++;
    const pasted = shape(v);
    const { error, output } = runPage({}).runMaker(pasted);
    if (v.valid) {
      const want = "https://freenet.org/open#" + v.local_path.slice("/v1/contract/web/".length);
      if (output !== want) {
        makerFailures++;
        console.error(
          `FAIL maker ${JSON.stringify(pasted.slice(0, 90))} (${v.note}): output ${JSON.stringify(output)} != ${JSON.stringify(want)}`,
        );
      }
    } else if (!error) {
      makerFailures++;
      console.error(
        `FAIL maker ${JSON.stringify(pasted.slice(0, 90))} (${v.note}): expected an error, got output ${JSON.stringify(output)}`,
      );
    }
  }
}
console.log(`${makerCases} maker vectors, ${makerFailures} failures`);

// --- Local-only apps (the Ghost Key vault): no try.freenet.org option, a
// note saying why instead, no ?via=browser redirect, and no browser link from
// the maker. Driven through a hashchange to an ordinary id too, since every
// one of these must be reset when the page moves off the vault. ---
const VAULT = "DLog47hEsrtuGT4N5XCeMBG45m4n1aWM89tBZXue2E1N";
const OTHER = "6FzSeAUKcqJrveKyU8RJgGKc5jRB1Z2juvxXtwTA4Em9";
let localOnlyFailures = 0;
{
  const check = (cond, what) => {
    if (!cond) {
      localOnlyFailures++;
      console.error(`FAIL local-only: ${what}`);
    }
  };
  const tryHidden = (el) => el("open-link-opt-try").style.display === "none";
  const noteShown = (el) =>
    el("open-link-local-only").style.display === "" &&
    el("open-link-local-only").textContent.length > 0;

  const page = runPage({ hash: "#" + VAULT + "/" });
  check(page.shown === "open-link-valid", `vault link showed ${page.shown}`);
  check(tryHidden(page.el), "try option visible for the vault");
  check(noteShown(page.el), "no local-only note for the vault");
  check(
    page.el("open-link-local").href === `http://127.0.0.1:7509/v1/contract/web/${VAULT}/`,
    `vault local button ${page.el("open-link-local").href}`,
  );
  check(page.navigate("#" + OTHER + "/") === "open-link-valid", "other id not valid");
  check(!tryHidden(page.el), "try option still hidden after leaving the vault");
  check(
    page.el("open-link-local-only").style.display === "none",
    "note still shown after leaving the vault",
  );
  check(!tryHidden(runPage({ hash: "#" + OTHER + "/" }).el), "try option hidden for an ordinary id");

  // ?via=browser must not send the vault to try.freenet.org, but must still
  // redirect an ordinary id reached by hashchange from it.
  const via = runPage({ hash: "#" + VAULT + "/", search: "?via=browser" });
  check(via.replacedTo === null, `via=browser redirected the vault to ${via.replacedTo}`);
  check(noteShown(via.el), "no local-only note for the vault under via=browser");
  via.navigate("#" + OTHER + "/");
  check(
    via.replacedTo === `https://try.freenet.org/v1/contract/web/${OTHER}/`,
    `via=browser after leaving the vault redirected to ${via.replacedTo}`,
  );

  // Maker: a try.freenet.org link preselects "for anyone". For the vault the
  // result must be the default link with the reason shown; for an ordinary id
  // it must stay a browser link with no note.
  const maker = runPage({});
  const vaultOut = maker.runMaker(`https://try.freenet.org/v1/contract/web/${VAULT}/`);
  check(
    vaultOut.output === `https://freenet.org/open#${VAULT}/`,
    `maker made ${vaultOut.output} for the vault`,
  );
  check(
    maker.el("open-maker-local-only").style.display === "" &&
      maker.el("open-maker-local-only").textContent.length > 0,
    "maker shows no reason for refusing a vault browser link",
  );
  const otherOut = maker.runMaker(`https://try.freenet.org/v1/contract/web/${OTHER}/`);
  check(
    otherOut.output === `https://freenet.org/open?via=browser#${OTHER}/`,
    `maker made ${otherOut.output} for an ordinary id`,
  );
  check(
    maker.el("open-maker-local-only").style.display === "none",
    "maker reason still shown for an ordinary id",
  );
}
console.log(`local-only checks, ${localOnlyFailures} failures`);

failures += viaFailures + makerFailures + localOnlyFailures;
if (vectors.length < 40) {
  console.error("vector file looks truncated");
  process.exit(1);
}
process.exit(failures ? 1 : 0);
