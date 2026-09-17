import { chromium } from "playwright";

const out = "D:/dev/cloudflare-auth/design/qa";
const url = "file:///D:/dev/cloudflare-auth/prototype/index.html";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 1024 } });

await page.goto(url, { waitUntil: "networkidle" });
await page.waitForTimeout(400);

// Apps nav hidden when signed out
const appsHidden = await page.locator("#navApps").isHidden();
console.log("apps-nav-hidden-when-out", appsHidden);

// Sign in
await page.fill("#loginEmail", "alice@example.com");
await page.fill("#loginPassword", "password123");
await page.click("#loginSubmit");
await page.waitForSelector("#view-profile.on", { timeout: 5000 });
await page.waitForTimeout(300);

const appsVisible = await page.locator("#navApps").isVisible();
console.log("apps-nav-visible-when-in", appsVisible);

// Profile CTA
await page.click("#gotoAppsBtn");
await page.waitForSelector("#view-apps.on");
await page.waitForTimeout(200);
await page.screenshot({ path: `${out}/apps-empty.png`, fullPage: false });

// Create app
await page.fill("#appName", "Acme Dashboard");
await page.fill("#appDesc", "Production web integration");
await page.click("#appSubmit");
await page.waitForTimeout(600);
const secretOn = await page.locator("#secretBanner.on").isVisible();
const listCount = await page.locator(".app-row").count();
console.log("secret-banner", secretOn, "rows", listCount);
await page.screenshot({ path: `${out}/apps-created.png`, fullPage: false });

// Create second + revoke flow
await page.fill("#appName", "Mobile App");
await page.click("#appSubmit");
await page.waitForTimeout(500);
page.on("dialog", async (d) => {
  console.log("dialog", d.message());
  await d.accept();
});
const revokeBtn = page.locator('[data-revoke]').last();
await revokeBtn.click();
await page.waitForTimeout(400);
const revoked = await page.locator(".app-row.revoked").count();
console.log("revoked-rows", revoked);
await page.screenshot({ path: `${out}/apps-list.png`, fullPage: false });

// Docs mention apps
await page.click("#navDocs");
await page.click("#cardDocsDev");
await page.waitForTimeout(200);
const hasAppDocs = await page.locator("text=App ID integration guide").count();
console.log("dev-docs-appid", hasAppDocs > 0);

await browser.close();
console.log("apps-ui-ok");
