import fs from "fs";

const files = [
  "D:/dev/cloudflare-auth/public/index.html",
  "D:/dev/cloudflare-auth/prototype/index.html",
];

/** Only exact broken fragments — no regex generics. */
const exact = [
  // HTML attributes / tags
  [
    'placeholder="鈥⑩€⑩€⑩€⑩€⑩€⑩€⑩€? required',
    'placeholder="••••••••" required',
  ],
  ['id="pfEmail">鈥?/div>', 'id="pfEmail">—</div>'],
  ['id="pfUsername">鈥?/div>', 'id="pfUsername">—</div>'],
  ['id="pfCreated">鈥?/div>', 'id="pfCreated">—</div>'],
  ['id="pfId">鈥?/div>', 'id="pfId">—</div>'],
  ["expire at 鈥?</p>", "expire at —.</p>"],
  ["<code id=\"secretValue\">sec_鈥?/code>", "<code id=\"secretValue\">sec_…</code>"],

  // JS strings that broke syntax (missing closing quote)
  ['(label || "Working鈥?);', '(label || "Working…");'],
  ['|| "鈥?;', '|| "—";'],
  ['setLoading(btn, true, "Creating鈥?);', 'setLoading(btn, true, "Creating…");'],
  ['setLoading(btn, true, "Signing in鈥?);', 'setLoading(btn, true, "Signing in…");'],

  // Toasts
  [
    'toast("Session expired 鈥?sign in again")',
    'toast("Session expired — sign in again")',
  ],
  [
    'toast("Session revoked 鈥?sign in for a new token")',
    'toast("Session revoked — sign in for a new token")',
  ],

  // Validation copy
  ["3鈥?2 letters, numbers, underscore", "3–32 letters, numbers, underscore"],
  ["3鈥?2 characters: letters", "3–32 characters: letters"],

  // Code samples / docs ellipsis that swallow following chars
  ["<code>sec_鈥?/code>", "<code>sec_…</code>"],
  ["<code>app_鈥?/code>", "<code>app_…</code>"],
  ['"id": "鈥?,', '"id": "…",'],
  ['"createdAt": "鈥?', '"createdAt": "…"'],
  ['"token": "eyJhbGciOi鈥?,', '"token": "eyJhbGciOi…",'],
  ['app_e14bab3b鈥?', 'app_e14bab3b…'],
  ['sec_7fd81b9627936e5c鈥?', 'sec_7fd81b9627936e5c…'],
  ['sec_7fd81b96鈥?', 'sec_7fd81b96…'],
  ['"appSecret": "sec_7fd81b96鈥?,', '"appSecret": "sec_7fd81b96…",'],
  ['"apps": [ { "id", "appId", "name", "status", "createdAt", 鈥?} ]', '"apps": [ { "id", "appId", "name", "status", "createdAt", … } ]'],
  ['{ user: { 鈥?}, "appId": "app_鈥? }', '{ user: { … }, "appId": "app_…" }'],
  ['{ app: { status: "revoked", 鈥?} }', '{ app: { status: "revoked", … } }'],

  // Prose em-dashes in docs (exact sentences)
  [
    "Isolated 鈥?you can rotate",
    "Isolated — you can rotate",
  ],
  [
    "keeps integrations isolated 鈥?you can rotate",
    "keeps integrations isolated — you can rotate",
  ],
  [
    "<strong>Create a developer account</strong> 鈥?sign up",
    "<strong>Create a developer account</strong> — sign up",
  ],
  [
    "<strong>Apply for an App ID</strong> 鈥?on the",
    "<strong>Apply for an App ID</strong> — on the",
  ],
  [
    "<strong>Store credentials</strong> 鈥?save",
    "<strong>Store credentials</strong> — save",
  ],
  [
    "<strong>Call register / login</strong> 鈥?send",
    "<strong>Call register / login</strong> — send",
  ],
  [
    "<strong>Use the user token</strong> 鈥?call",
    "<strong>Use the user token</strong> — call",
  ],
  [
    "<strong>Operate the app</strong> 鈥?rotate",
    "<strong>Operate the app</strong> — rotate",
  ],
  ["Step 1 鈥?Apply", "Step 1 — Apply"],
  ["Step 2 鈥?Configure", "Step 2 — Configure"],
  ["Step 3 鈥?Call", "Step 3 — Call"],
  ["Step 4 鈥?Use", "Step 4 — Use"],
  ["Step 5 鈥?Lifecycle", "Step 5 — Lifecycle"],
  ["copy App ID + Secret 鈥?immediately", "copy App ID + Secret — immediately"],
  ["Sign in 鈫?<strong>Applications</strong> 鈫?<strong>Create application</strong> 鈫?copy",
   "Sign in → <strong>Applications</strong> → <strong>Create application</strong> → copy"],
  ["# 鈫?{", "# → {"],
  ["# 鈫?201", "# → 201"],
  ["# 鈫?200", "# → 200"],
  ["data.token 鈫?give", "data.token → give"],
  ["<code>app_鈥?</code> and", "<code>app_…</code> and"],
  ["one-time <code>sec_鈥?</code>", "one-time <code>sec_…</code>"],
  ["Old <code>sec_鈥?</code>", "Old <code>sec_…</code>"],
  ["unrelated to App ID 鈥?fix", "Unrelated to App ID — fix"],
  ["Unrelated to App ID 鈥?fix", "Unrelated to App ID — fix"],
  ["Create app 鈥?returns", "Create app — returns"],
  [
    "invalid_credentials</code> 鈥?the API",
    "invalid_credentials</code> — the API",
  ],
  [
    "<strong>Email</strong> 鈥?used to sign in",
    "<strong>Email</strong> — used to sign in",
  ],
  [
    "<strong>Username</strong> 鈥?3鈥?2 characters",
    "<strong>Username</strong> — 3–32 characters",
  ],
  [
    "<strong>Password</strong> 鈥?at least 8",
    "<strong>Password</strong> — at least 8",
  ],
  [
    "already taken, you鈥檒l see",
    "already taken, you’ll see",
  ],
  [
    "Try a different value 鈥?we never",
    "Try a different value — we never",
  ],
  [
    "intentional 鈥?it does not",
    "intentional — it does not",
  ],
  [
    "<strong>Session status</strong> 鈥?a green",
    "<strong>Session status</strong> — a green",
  ],
  [
    "<strong>API Base URL</strong> 鈥?only relevant",
    "<strong>API Base URL</strong> — only relevant",
  ],
  [
    "revokes that session 鈥?the same token",
    "revokes that session — the same token",
  ],
  [
    "password hash 鈥?never your password",
    "password hash — never your password",
  ],
  [
    "this version 鈥?register again",
    "this version — register again",
  ],
  [
    "use {{APP_NAME}} 鈥?integrate",
    "use {{APP_NAME}} — integrate",
  ],
  [
    "user token</strong> 鈥?never the App Secret",
    "user token</strong> — never the App Secret",
  ],
  ["characters</p> 鈥?password", "characters</p> · password"],
  ["password 鈮?8 characters", "password ≥ 8 characters"],
  ["keep secrets 鈥?server-side", "Keep secrets server-side"],
];

for (const file of files) {
  if (!fs.existsSync(file)) continue;
  let text = fs.readFileSync(file, "utf8");
  let n = 0;
  for (const [from, to] of exact) {
    if (text.includes(from)) {
      const parts = text.split(from);
      n += parts.length - 1;
      text = parts.join(to);
    }
  }
  fs.writeFileSync(file, text, "utf8");
  console.log("fixed", n, "fragments in", file);

  const ok =
    text.includes('(label || "Working…")') &&
    text.includes('id="pfEmail">—</div>') &&
    text.includes('placeholder="••••••••" required') &&
    !text.includes("Working鈥") &&
    !text.includes("set — token") &&
    text.includes("<link href=") &&
    text.includes("<p class=\"form-foot\">");
  console.log("verify", file, ok);
}
