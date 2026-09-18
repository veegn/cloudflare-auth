import { chromium } from "playwright";

const base = "https://auth.dayti.de";

// Create a developer + app with description via API
const email = `ui_${Date.now()}@example.com`;
const username = `ui${Date.now().toString().slice(-8)}`;
const regRes = await fetch(`${base}/auth/register`, {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ email, username, password: "password123" }),
});
const reg = await regRes.json();
const token = reg.token;

const appRes = await fetch(`${base}/apps`, {
  method: "POST",
  headers: {
    "content-type": "application/json",
    Authorization: `Bearer ${token}`,
  },
  body: JSON.stringify({
    name: "Acme Dashboard",
    description: "Production analytics console for the Acme team.",
    redirectUris: ["https://app.example.com/callback"],
  }),
});
const appData = await appRes.json();
const clientId = appData.app.appId;

const authUrl =
  `${base}/?client_id=${encodeURIComponent(clientId)}` +
  `&redirect_uri=${encodeURIComponent("https://app.example.com/callback")}` +
  `&response_type=code&state=demo123`;

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 1024 } });
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));

await page.goto(authUrl, { waitUntil: "networkidle", timeout: 60000 });
await page.waitForTimeout(600);

const title = await page.locator("#authzTitle").innerText().catch(() => "");
const desc = await page.locator("#authzDesc").innerText().catch(() => "");
const lede = await page.locator("#loginLede").innerText().catch(() => "");
const bannerOn = await page.locator("#authzBanner.on").isVisible().catch(() => false);
const loginOn = await page.locator("#view-login.on").isVisible().catch(() => false);

// layout metrics
const formBox = await page.locator("#loginForm").boundingBox().catch(() => null);
const bannerBox = await page.locator("#authzBanner").boundingBox().catch(() => null);
const mainBox = await page.locator(".main").boundingBox().catch(() => null);

console.log({
  clientId,
  authUrl,
  title,
  desc,
  lede,
  bannerOn,
  loginOn,
  formBox,
  bannerBox,
  mainBox,
  errors,
});

await page.screenshot({
  path: "D:/dev/cloudflare-auth/design/qa/login-authz-pc.png",
  fullPage: false,
});

// also plain login without authorize
await page.goto(base + "/", { waitUntil: "networkidle", timeout: 60000 });
await page.waitForTimeout(400);
await page.screenshot({
  path: "D:/dev/cloudflare-auth/design/qa/login-pc-centered.png",
  fullPage: false,
});
const plainLede = await page.locator("#loginLede").innerText().catch(() => "");
const plainBanner = await page.locator("#authzBanner.on").isVisible().catch(() => false);
console.log({ plainLede, plainBanner });

await browser.close();
console.log("done");
