import { chromium } from "playwright";

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 1024 } });
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
page.on("console", (msg) => {
  if (msg.type() === "error") errors.push(msg.text());
});

const res = await page.goto("https://auth.dayti.de/", {
  waitUntil: "networkidle",
  timeout: 60000,
});
await page.waitForTimeout(500);
const loginVisible = await page.locator("#view-login.on").isVisible().catch(() => false);
const title = await page.title();
const h1 = await page.locator("#view-login h1").innerText().catch(() => "");
const brand = await page.locator(".brand span").innerText().catch(() => "");
const apiBase = await page.locator("#apiBase").innerText().catch(() => "");

console.log({
  status: res && res.status(),
  title,
  brand,
  loginVisible,
  h1,
  apiBase,
  errors,
});

await page.screenshot({ path: "D:/dev/cloudflare-auth/design/qa/prod-home-fixed.png" });
await browser.close();
