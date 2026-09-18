import { chromium } from "playwright";

const base = "https://auth.dayti.de";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 1024 } });
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));

// 1) plain login
await page.goto(base + "/", { waitUntil: "networkidle", timeout: 60000 });
await page.waitForTimeout(500);
const lede = await page.locator("#loginLede").innerText().catch(() => "");
const h1 = await page.locator("#loginTitle").innerText().catch(() => "");
const brandVisible = await page.locator(".auth-brand").isVisible().catch(() => false);
const card = await page.locator("#view-login.auth-wrap").boundingBox().catch(() => null);
console.log({ h1, lede, brandVisible, card, errors });
await page.screenshot({ path: "D:/dev/cloudflare-auth/design/qa/g-login.png" });

// 2) register
await page.click("#toRegister");
await page.waitForTimeout(300);
await page.screenshot({ path: "D:/dev/cloudflare-auth/design/qa/g-register.png" });

// 3) sign in and profile
await page.click("#toLogin");
await page.fill("#loginEmail", "alice@example.com");
await page.fill("#loginPassword", "password123");
// may fail if alice not in prod db - try register instead
const regEmail = `g_${Date.now()}@example.com`;
const regUser = `g${Date.now().toString().slice(-8)}`;
await page.click("#toRegister");
await page.fill("#regEmail", regEmail);
await page.fill("#regUsername", regUser);
await page.fill("#regPassword", "password123");
await page.fill("#regConfirm", "password123");
await page.click("#regSubmit");
await page.waitForSelector("#view-profile.on", { timeout: 15000 }).catch(() => {});
await page.waitForTimeout(800);
await page.screenshot({ path: "D:/dev/cloudflare-auth/design/qa/g-profile.png" });

await page.click("#gotoAppsBtn");
await page.waitForTimeout(800);
await page.screenshot({ path: "D:/dev/cloudflare-auth/design/qa/g-apps.png" });

// 4) docs
await page.click("#navDocs");
await page.waitForTimeout(300);
await page.screenshot({ path: "D:/dev/cloudflare-auth/design/qa/g-docs.png" });

// 5) authorize third-party
const token = await page.evaluate(() => localStorage.getItem("cfa_token"));
let authUrl = base + "/";
if (token) {
  const appRes = await fetch(base + "/apps", {
    method: "POST",
    headers: { "content-type": "application/json", Authorization: `Bearer ${token}` },
    body: JSON.stringify({
      name: "Acme Dashboard",
      description: "Production analytics console for the Acme team.",
      redirectUris: ["https://app.example.com/callback"],
    }),
  });
  const app = await appRes.json();
  if (app.app) {
    authUrl =
      `${base}/?client_id=${app.app.appId}` +
      `&redirect_uri=${encodeURIComponent("https://app.example.com/callback")}` +
      `&response_type=code&state=x`;
  }
}
await page.goto(authUrl, { waitUntil: "networkidle", timeout: 60000 });
await page.waitForTimeout(600);
await page.screenshot({ path: "D:/dev/cloudflare-auth/design/qa/g-authz.png" });
const authzTitle = await page.locator("#authzTitle").innerText().catch(() => "");
console.log({ authzTitle, errors });
await browser.close();
console.log("done");
