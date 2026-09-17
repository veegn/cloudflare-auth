import { chromium } from "playwright";

const out = "D:/dev/cloudflare-auth/design/qa";
const url = "file:///D:/dev/cloudflare-auth/prototype/index.html";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 1024 } });

await page.goto(url, { waitUntil: "networkidle" });
await page.waitForTimeout(500);
await page.screenshot({ path: `${out}/login.png`, fullPage: false });

// Docs landing from sidebar
await page.click("#navDocs");
await page.waitForTimeout(250);
await page.screenshot({ path: `${out}/docs-home.png`, fullPage: false });

// Developer docs
await page.click("#cardDocsDev");
await page.waitForTimeout(250);
await page.screenshot({ path: `${out}/docs-dev.png`, fullPage: false });

// User docs
await page.click("#navDocs");
await page.waitForTimeout(150);
await page.click("#cardDocsUser");
await page.waitForTimeout(250);
await page.screenshot({ path: `${out}/docs-user.png`, fullPage: false });

// Login docs link
await page.click("#navAuth");
await page.waitForTimeout(200);
const docsLink = page.locator("#view-login .js-docs");
await docsLink.click();
await page.waitForTimeout(200);
const docsVisible = await page.locator("#view-docs.on").isVisible();
console.log("login-docs-link-ok", docsVisible);

// Back to account
await page.click("#navAuth");
await page.waitForTimeout(200);
const loginVisible = await page.locator("#view-login.on").isVisible();
console.log("nav-auth-ok", loginVisible);

await browser.close();
console.log("screenshots ok");
