import { chromium } from "playwright";

const targets = [
  { name: "local-public", url: "file:///D:/dev/cloudflare-auth/public/index.html" },
];

const browser = await chromium.launch();
for (const t of targets) {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1024 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  await page.goto(t.url, { waitUntil: "networkidle", timeout: 60000 });
  await page.waitForTimeout(400);
  const loginVisible = await page.locator("#view-login.on").isVisible().catch(() => false);
  const title = await page.title();
  const h1 = await page.locator("#view-login h1").innerText().catch(() => "");
  console.log(t.name, { title, loginVisible, h1, errors });
  await page.screenshot({ path: `D:/dev/cloudflare-auth/design/qa/fix-${t.name}.png` });
  await page.close();
}
await browser.close();
console.log("ok");
