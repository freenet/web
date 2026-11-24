// Loads the real /open page script in headless Chromium and asserts that NO
// request to 127.0.0.1 or localhost is made on page load, for desktop and
// mobile user agents. Chrome's Local Network Access shows a permission prompt
// for such a request from a public origin, so any load-time request here is a
// regression (a visitor would be prompted before doing anything).
//
// Needs the `playwright` package and a Chromium (npx playwright install chromium).
// Usage: node hugo-site/tests/open-link-no-probe.browser.mjs

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
// The shortcode is plain HTML + script (no Hugo template syntax), so it can be
// served as-is inside a minimal page.
const shortcode = readFileSync(
  join(here, "../themes/freenet/layouts/shortcodes/open-link.html"),
  "utf8",
);
const PAGE = `<!doctype html><html><head><meta charset="utf-8"></head><body>
<p id="open-link-lede"></p>${shortcode}</body></html>`;
const ORIGIN = "https://freenet.org";
const ID = "raAqMhMG7KUpXBU2SxgCQ3Vh4PYjttxdSWd9ftV7RLv";

const agents = {
  desktop: {
    userAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/131.0 Safari/537.36",
    isMobile: false,
    hasTouch: false,
  },
  android: {
    userAgent:
      "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/131.0 Mobile Safari/537.36",
    isMobile: true,
    hasTouch: true,
  },
};

const isLocal = (url) => /^[a-z]+:\/\/(127\.\d+\.\d+\.\d+|localhost|\[::1\])(:\d+)?(\/|$)/i.test(url);

let failures = 0;
const browser = await chromium.launch();
try {
  for (const [name, ctxOpts] of Object.entries(agents)) {
    for (const search of ["", "?via=app", "?via=local"]) {
      const context = await browser.newContext(ctxOpts);
      const page = await context.newPage();
      const requested = [];
      await context.route("**/*", (route) => {
        const url = route.request().url();
        requested.push(url);
        if (url === `${ORIGIN}/open${search}`) {
          return route.fulfill({ contentType: "text/html", body: PAGE });
        }
        return route.abort();
      });
      await page.goto(`${ORIGIN}/open${search}#${ID}/`);
      await page.waitForSelector("#open-link-valid", { state: "visible" });
      // Longer than the old 1.5 s probe timeout, so a delayed probe is caught.
      await page.waitForTimeout(2000);

      const local = requested.filter(isLocal);
      if (local.length) {
        failures++;
        console.error(`FAIL ${name} ${search || "(none)"}: loopback requests ${JSON.stringify(local)}`);
      }
      const primary = await page.$eval("#open-link-primary-slot a", (a) => a.id);
      if (primary !== (search === "?via=app" ? "open-link-scheme" : "open-link-try")) {
        if (search !== "?via=local") {
          failures++;
          console.error(`FAIL ${name} ${search || "(none)"}: primary is ${primary}`);
        }
      }
      // Mobile: the local option stays out of the always-visible area.
      const visibleLocal = await page.isVisible("#open-link-secondary-slot #open-link-local");
      if (ctxOpts.isMobile === visibleLocal && search === "") {
        failures++;
        console.error(`FAIL ${name}: local option visible=${visibleLocal}`);
      }
      await context.close();
    }
  }
} finally {
  await browser.close();
}
console.log(`no-probe browser checks, ${failures} failures`);
process.exit(failures ? 1 : 0);
