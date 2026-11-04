// Headless screenshot of a mock-up page with Playwright and its own Chromium, called by
// screenshot.sh. Prints the errors and warnings of the browser console, and exits with an
// error when the page has one, so that a broken script never goes unnoticed.
//
//   node screenshot.cjs <url> <output.png> <width> <height | full> [selector]
//
// With a selector (`#section-3 .window`), only the first element it matches is captured,
// and the texts inside it that are cut (wider than their box) or wrapped (on several lines)
// are listed, with their width: the measures the screenshot alone doesn't give. SCALE sets
// the scale of the screen (`deviceScaleFactor`, 1 by default, the owner's screen at 100 %).
//
// Playwright is installed once, outside the projects (about 150 MB for Chromium):
//   mkdir -p ~/.local/share/mockup-tools && cd ~/.local/share/mockup-tools
//   npm init -y && npm install playwright && npx playwright install chromium
// PLAYWRIGHT_HOME points to another installation folder.
const path = require("node:path");

const home = process.env.PLAYWRIGHT_HOME || path.join(process.env.HOME, ".local/share/mockup-tools");
const { chromium } = require(path.join(home, "node_modules/playwright"));

// Runs in the page: the texts of `root` that are cut or wrapped.
function measureTexts(root) {
  const found = [];
  const shorten = text => (text.length > 60 ? `${text.slice(0, 57)}…` : text);
  for (const element of root.querySelectorAll("*")) {
    const own = [...element.childNodes].filter(node => node.nodeType === Node.TEXT_NODE && node.textContent.trim());
    if (own.length === 0 || element.getClientRects().length === 0) continue;
    const text = shorten(element.textContent.trim().replace(/\s+/g, " "));
    const name = element.tagName.toLowerCase() + (element.className && typeof element.className === "string" ? `.${element.className.trim().split(/\s+/).join(".")}` : "");
    if (element.scrollWidth > element.clientWidth + 1 && getComputedStyle(element).overflowX !== "visible") {
      found.push(`cut: ${name} "${text}" (${element.scrollWidth} px in ${element.clientWidth} px)`);
    }
    // The lines on screen, against those its own line breaks make when its white space keeps them (a log).
    const lines = new Set();
    let breaks = 0;
    const keepsBreaks = /^pre/.test(getComputedStyle(element).whiteSpace);
    for (const node of own) {
      const range = document.createRange();
      range.selectNodeContents(node);
      for (const rect of range.getClientRects()) {
        if (rect.width > 0) lines.add(Math.round(rect.top));
      }
      breaks += keepsBreaks ? node.textContent.split("\n").filter(line => line.trim()).length : 1;
    }
    if (lines.size > Math.max(breaks, 1)) {
      const written = keepsBreaks ? `, ${breaks} written` : "";
      found.push(`wrapped: ${name} "${text}" (${lines.size} lines${written}, in ${Math.round(element.getBoundingClientRect().width)} px)`);
    }
  }
  return found;
}

async function main() {
  const [url, output, width, height, selector] = process.argv.slice(2);
  const fullPage = height === "full";
  const scale = Number(process.env.SCALE || 1);
  const browser = await chromium.launch();
  let errors = 0;
  try {
    const page = await browser.newPage({
      viewport: { width: Number(width), height: fullPage ? 900 : Number(height) },
      deviceScaleFactor: scale,
    });
    page.on("console", message => {
      if (message.type() === "error" || message.type() === "warning") {
        console.error(`console ${message.type()}: ${message.text()}`);
        if (message.type() === "error") errors += 1;
      }
    });
    page.on("pageerror", error => {
      console.error(`page error: ${error.message}`);
      errors += 1;
    });
    page.on("requestfailed", request => {
      console.error(`request failed: ${request.url()}`);
      errors += 1;
    });
    await page.goto(url, { waitUntil: "networkidle" });
    await page.evaluate(() => document.fonts.ready);
    if (selector) {
      const target = page.locator(selector).first();
      await target.screenshot({ path: output });
      for (const line of await target.evaluate(measureTexts)) console.log(line);
    } else {
      await page.screenshot({ path: output, fullPage });
    }
  } finally {
    await browser.close();
  }
  console.log(output);
  if (errors > 0) process.exitCode = 2;
}

main().catch(error => {
  console.error(error.message);
  process.exitCode = 1;
});
