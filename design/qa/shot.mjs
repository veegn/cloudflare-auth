import { chromium } from "playwright";

const out = "D:/dev/cloudflare-auth/design/qa";
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 1024 } });

await page.goto("http://127.0.0.1:8765/", { waitUntil: "networkidle" });
await page.waitForTimeout(400);
await page.screenshot({ path: `${out}/login.png`, fullPage: false });

await page.click("#toRegister");
await page.waitForTimeout(200);
await page.screenshot({ path: `${out}/register.png`, fullPage: false });

await page.click("#toLogin");
await page.fill("#loginEmail", "alice@example.com");
await page.fill("#loginPassword", "password123");
await page.click("#loginSubmit");
await page.waitForSelector("#view-profile.on", { timeout: 5000 });
await page.waitForTimeout(300);
await page.screenshot({ path: `${out}/profile.png`, fullPage: false });

await browser.close();
console.log("screenshots ok");
