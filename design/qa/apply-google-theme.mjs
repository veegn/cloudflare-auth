import fs from "fs";

const htmlPath = "D:/dev/cloudflare-auth/public/index.html";
const cssPath = "D:/dev/cloudflare-auth/design/qa/google-theme.css";

let html = fs.readFileSync(htmlPath, "utf8");
const css = fs.readFileSync(cssPath, "utf8");

html = html.replace(
  /<link href="https:\/\/fonts\.googleapis\.com\/css2\?family=Inter[^"]*" rel="stylesheet" \/>/,
  '<link href="https://fonts.googleapis.com/css2?family=Roboto:wght@400;500;700&family=Roboto+Mono:wght@400;500&display=swap" rel="stylesheet" />'
);

const styleStart = html.indexOf("  <style>");
const styleEnd = html.indexOf("  </style>");
if (styleStart < 0 || styleEnd < 0) {
  console.error("style block not found");
  process.exit(1);
}

html =
  html.slice(0, styleStart) +
  "  <style>\n" +
  css +
  "\n" +
  html.slice(styleEnd);

fs.writeFileSync(htmlPath, html, "utf8");
console.log("style replaced, html bytes", html.length);

// sanity checks
const ok =
  html.includes("--accent: #1a73e8") &&
  html.includes("Roboto") &&
  html.includes(".auth-wrap") &&
  html.includes("auth-mode") &&
  html.includes('id="view-login"') &&
  html.includes("tryCompleteAuthorize");
console.log("sanity", ok);
