/**
 * cloudflare-auth console i18n (en / zh-CN)
 * Keys are stable; English is the source string fallback.
 */
(function () {
  const DICT = {
    en: {
      "meta.title": "{{APP_NAME}}",
      "nav.account": "Account",
      "nav.docs": "Docs",
      "nav.apps": "Applications",
      "nav.signOut": "Sign out",
      "lang.en": "EN",
      "lang.zh": "中文",

      "auth.signIn": "Sign in",
      "auth.signInLede": "Use your {{APP_NAME}} account",
      "auth.email": "Email",
      "auth.password": "Password",
      "auth.newHere": "New here?",
      "auth.createAccount": "Create an account",
      "auth.lookingDocs": "Looking for integration details?",
      "auth.readDocs": "Read the docs",
      "auth.createTitle": "Create your account",
      "auth.createLede": "Continue to {{APP_NAME}}",
      "auth.username": "Username",
      "auth.confirmPassword": "Confirm password",
      "auth.createSubmit": "Create account",
      "auth.haveAccount": "Already have an account?",
      "auth.newTo": "New to {{APP_NAME}}?",
      "auth.howAccounts": "How accounts work",
      "auth.ph.email": "name@example.com",
      "auth.ph.password": "••••••••",
      "auth.ph.username": "alice",
      "auth.ph.passwordMin": "At least 8 characters",
      "auth.ph.repeatPassword": "Repeat password",

      "authz.thirdParty": "Third-party application",
      "authz.application": "Application",
      "authz.continue": "Continue",
      "authz.confirmLede": "You are signed in. Confirm to continue to the application.",
      "authz.anotherAccount": "Use another account",
      "authz.notYou": "Not you?",
      "authz.requesting": "Requesting sign-in",
      "authz.error": "Authorization error",
      "authz.unavailable": "Sign-in unavailable",
      "authz.continueAs": "Continue as {{name}}",
      "authz.continueTo": "Continue to {{name}}?",
      "authz.signInContinue": "Sign in to continue to {{name}}",
      "authz.createFor": "Create an account for {{name}}",
      "authz.useAccount": "Use your {{APP_NAME}} account",
      "authz.continueApp": "Continue to {{APP_NAME}}",

      "profile.title": "Profile",
      "profile.lede": "Your account and session details.",
      "profile.edit": "Edit profile",
      "profile.newAvatar": "New avatar",
      "profile.newAvatarTitle": "Generate a new default avatar",
      "profile.uploadPhoto": "Upload photo",
      "profile.uploadPhotoTitle": "Upload photo (server crop+compress)",
      "profile.accountDetails": "Account details",
      "profile.updateHint": "Update your email, username, or password.",
      "profile.currentPassword": "Current password",
      "profile.newPassword": "New password",
      "profile.saveChanges": "Save changes",
      "profile.cancel": "Cancel",
      "profile.session": "Session",
      "profile.sessionStatus": "Session status",
      "profile.apiBaseHint": "Your authentication API base URL",
      "profile.apiBase": "API base",
      "profile.activeExpires": "Active · Expires in {{label}}",
      "profile.willExpire": "This session will expire at {{time}}.",
      "profile.copyBase": "Copy",
      "profile.revokeSession": "Revoke session",
      "profile.regenToken": "Get a new token",
      "profile.signOutHere": "Sign out from this device.",

      "apps.title": "Applications",
      "apps.lede": "Register apps to obtain App ID / App Secret for integration.",
      "apps.formNew": "Register a new app",
      "apps.formNewHint": "Create an App ID, then register redirect URIs for redirect login.",
      "apps.formEdit": "Edit application",
      "apps.formEditHint": "Update name, description, or redirect URIs. App Secret is not shown here.",
      "apps.name": "App name",
      "apps.description": "Description",
      "apps.redirectUris": "Redirect URIs",
      "apps.redirectHint": "(optional, one per line)",
      "apps.redirectHelp": "Required for redirect login. Exact match; max 20 URIs.",
      "apps.iconUrl": "Icon URL",
      "apps.iconOptional": "(optional)",
      "apps.iconHelp": "HTTPS image on sign-in / consent. Empty = generated icon.",
      "apps.uploadIcon": "Upload icon",
      "apps.uploadIconHint": "Center-cropped & compressed on server",
      "apps.create": "Create application",
      "apps.save": "Save changes",
      "apps.yourApps": "Your apps",
      "apps.empty": "No applications yet. Create one to get an App ID.",
      "apps.edit": "Edit",
      "apps.rotate": "Rotate secret",
      "apps.revoke": "Revoke",
      "apps.active": "Active",
      "apps.revoked": "Revoked",
      "apps.copy": "Copy",
      "apps.noDesc": "No description",

      "docs.title": "Documentation",
      "docs.lede": "Choose the guide that matches how you use {{APP_NAME}} — integrate the API as a developer, or manage your own account as a user.",
      "docs.forDevelopers": "For developers",
      "docs.apiIntegration": "API & integration",
      "docs.apiIntegrationDesc": "Endpoints, auth headers, request bodies, error codes, and how to wire register / login / session into your app.",
      "docs.openDev": "Open developer docs",
      "docs.forUsers": "For users",
      "docs.accountSessions": "Account & sessions",
      "docs.accountSessionsDesc": "Create an account, sign in, understand session expiry, sign out safely, and know what profile data is stored.",
      "docs.openUser": "Open user guide",
      "docs.devTitle": "Developer guide",
      "docs.devLede": "Integrate {{APP_NAME}} into your app with email/password accounts and revocable sessions.",
      "docs.userTitle": "User guide",
      "docs.userLede": "How accounts, sessions, and applications work in {{APP_NAME}}.",
      "docs.quickStart": "Quick start",
      "docs.baseUrl": "Base URL for this environment:",
      "docs.localWorker": "Local Worker (during development):",
      "docs.step1": "Register a user with <code>POST /auth/register</code>.",
      "docs.step2": "Receive a JWT <code>token</code> and store it on the client.",
      "docs.step3": "Call protected routes with <code>Authorization: Bearer &lt;token&gt;</code>.",
      "docs.step4": "Revoke the session with <code>POST /auth/logout</code> when done.",
      "docs.apiRef": "API reference",
      "docs.th.method": "Method",
      "docs.th.path": "Path",
      "docs.th.auth": "Auth",
      "docs.th.desc": "Description",
      "docs.needApi": "Need the API?",
      "docs.needApiBody": "If you are building an app on top of {{APP_NAME}}, switch to the <a href=\"#\" class=\"js-docs-dev\">developer guide</a> for endpoints and examples.",
      "docs.crumb": "Docs",
      "docs.crumbDevelopers": "Developers",
      "docs.crumbUsers": "Users",
      "docs.signOutHow": "Sign out",
      "docs.troubleshooting": "Troubleshooting",
      "docs.youSee": "You see",
      "docs.whatToDo": "What to do",

      "toast.sessionExpired": "Session expired — sign in again",
      "toast.signedOut": "Signed out",
      "toast.newAvatar": "New avatar generated",
      "toast.avatarUploaded": "Avatar uploaded",
      "toast.avatarFail": "Could not refresh avatar",
      "toast.avatarUploadFail": "Avatar upload failed",
      "toast.profileUpdated": "Profile updated",
      "toast.appsLoadFail": "Failed to load apps",
      "toast.appNotFound": "App not found",
      "toast.editCancelled": "Edit cancelled",
      "toast.uploadIconFirst": "Create the app first, then upload an icon",
      "toast.iconUploaded": "App icon uploaded",
      "toast.iconUploadFail": "Icon upload failed",
      "toast.appUpdated": "Application updated",
      "toast.appIdIssued": "App ID issued: {{id}}",
      "toast.appIdCopied": "App ID copied",
      "toast.secretRotated": "Secret rotated",
      "toast.rotateFail": "Rotate failed",
      "toast.appRevoked": "Application revoked",
      "toast.revokeFail": "Revoke failed",
      "toast.secretCopied": "Secret copied",
      "toast.apiBaseCopied": "API Base URL copied",
      "toast.sessionRevoked": "Session revoked — sign in for a new token",
      "toast.sessionRevokeFail": "Could not revoke session",
      "toast.signInApps": "Sign in to manage applications",
      "toast.switchAccount": "Sign in with another account",
      "toast.secretTitle": "Store this secret now",
      "toast.working": "Working…",
      "err.emailInvalid": "Email is invalid",
      "err.usernameRequired": "Username is required",
      "err.passwordRequired": "Password is required",
      "err.passwordMin": "Password must be at least 8 characters",
      "err.passwordMatch": "Passwords do not match",
      "err.appNameRequired": "App name is required",
      "err.currentPassword": "Current password is required",
      "err.newPasswordMin": "New password must be at least 8 characters",
    },
    zh: {
      "meta.title": "{{APP_NAME}}",
      "nav.account": "账户",
      "nav.docs": "文档",
      "nav.apps": "应用",
      "nav.signOut": "退出登录",
      "lang.en": "EN",
      "lang.zh": "中文",

      "auth.signIn": "登录",
      "auth.signInLede": "使用你的 {{APP_NAME}} 账户",
      "auth.email": "邮箱",
      "auth.password": "密码",
      "auth.newHere": "还没有账号？",
      "auth.createAccount": "创建账户",
      "auth.lookingDocs": "需要接入文档？",
      "auth.readDocs": "阅读文档",
      "auth.createTitle": "创建账户",
      "auth.createLede": "继续使用 {{APP_NAME}}",
      "auth.username": "用户名",
      "auth.confirmPassword": "确认密码",
      "auth.createSubmit": "创建账户",
      "auth.haveAccount": "已有账号？",
      "auth.newTo": "初次使用 {{APP_NAME}}？",
      "auth.howAccounts": "账户说明",
      "auth.ph.email": "name@example.com",
      "auth.ph.password": "••••••••",
      "auth.ph.username": "alice",
      "auth.ph.passwordMin": "至少 8 个字符",
      "auth.ph.repeatPassword": "再次输入密码",

      "authz.thirdParty": "第三方应用",
      "authz.application": "应用",
      "authz.continue": "继续",
      "authz.confirmLede": "你已登录。确认后继续前往该应用。",
      "authz.anotherAccount": "使用其他账户",
      "authz.notYou": "不是你？",
      "authz.requesting": "请求登录",
      "authz.error": "授权错误",
      "authz.unavailable": "无法登录",
      "authz.continueAs": "以 {{name}} 继续",
      "authz.continueTo": "继续前往 {{name}}？",
      "authz.signInContinue": "登录后继续前往 {{name}}",
      "authz.createFor": "为 {{name}} 创建账户",
      "authz.useAccount": "使用你的 {{APP_NAME}} 账户",
      "authz.continueApp": "继续使用 {{APP_NAME}}",

      "profile.title": "个人资料",
      "profile.lede": "你的账户与会话信息。",
      "profile.edit": "编辑资料",
      "profile.newAvatar": "新头像",
      "profile.newAvatarTitle": "生成新的默认头像",
      "profile.uploadPhoto": "上传照片",
      "profile.uploadPhotoTitle": "上传照片（服务端裁剪压缩）",
      "profile.accountDetails": "账户详情",
      "profile.updateHint": "更新邮箱、用户名或密码。",
      "profile.currentPassword": "当前密码",
      "profile.newPassword": "新密码",
      "profile.saveChanges": "保存修改",
      "profile.cancel": "取消",
      "profile.session": "会话",
      "profile.sessionStatus": "会话状态",
      "profile.apiBaseHint": "你的鉴权 API 地址",
      "profile.apiBase": "API 地址",
      "profile.activeExpires": "有效 · 剩余 {{label}}",
      "profile.willExpire": "此会话将于 {{time}} 过期。",
      "profile.copyBase": "复制",
      "profile.revokeSession": "吊销会话",
      "profile.regenToken": "获取新令牌",
      "profile.signOutHere": "在此设备退出登录。",

      "apps.title": "应用",
      "apps.lede": "注册应用以获取接入用的 App ID / App Secret。",
      "apps.formNew": "注册新应用",
      "apps.formNewHint": "创建 App ID，并为重定向登录登记 redirect URI。",
      "apps.formEdit": "编辑应用",
      "apps.formEditHint": "更新名称、描述或 redirect URI。此处不显示 App Secret。",
      "apps.name": "应用名称",
      "apps.description": "描述",
      "apps.redirectUris": "Redirect URIs",
      "apps.redirectHint": "（可选，每行一个）",
      "apps.redirectHelp": "重定向登录需要。精确匹配，最多 20 条。",
      "apps.iconUrl": "图标 URL",
      "apps.iconOptional": "（可选）",
      "apps.iconHelp": "登录/授权页展示的 HTTPS 图片。留空则使用生成图标。",
      "apps.uploadIcon": "上传图标",
      "apps.uploadIconHint": "服务端中心裁剪并压缩",
      "apps.create": "创建应用",
      "apps.save": "保存修改",
      "apps.yourApps": "你的应用",
      "apps.empty": "暂无应用。创建一个以获取 App ID。",
      "apps.edit": "编辑",
      "apps.rotate": "轮换 Secret",
      "apps.revoke": "吊销",
      "apps.active": "正常",
      "apps.revoked": "已吊销",
      "apps.copy": "复制",
      "apps.noDesc": "暂无描述",

      "docs.title": "文档",
      "docs.lede": "选择适合你的指南 —— 开发者接入 API，或用户管理自己的账户。",
      "docs.forDevelopers": "面向开发者",
      "docs.apiIntegration": "API 与接入",
      "docs.apiIntegrationDesc": "接口、鉴权头、请求体、错误码，以及如何把注册 / 登录 / 会话接入你的应用。",
      "docs.openDev": "打开开发者文档",
      "docs.forUsers": "面向用户",
      "docs.accountSessions": "账户与会话",
      "docs.accountSessionsDesc": "创建账户、登录、理解会话过期、安全退出，以及了解会存储哪些资料数据。",
      "docs.openUser": "打开用户指南",
      "docs.devTitle": "开发者指南",
      "docs.devLede": "使用邮箱密码账户与可吊销会话，将 {{APP_NAME}} 接入你的应用。",
      "docs.userTitle": "用户指南",
      "docs.userLede": "{{APP_NAME}} 中账户、会话与应用的工作方式。",
      "docs.quickStart": "快速开始",
      "docs.baseUrl": "当前环境 Base URL：",
      "docs.localWorker": "本地 Worker（开发时）：",
      "docs.step1": "调用 <code>POST /auth/register</code> 注册用户。",
      "docs.step2": "获取 JWT <code>token</code> 并保存在客户端。",
      "docs.step3": "请求受保护接口时携带 <code>Authorization: Bearer &lt;token&gt;</code>。",
      "docs.step4": "结束后用 <code>POST /auth/logout</code> 吊销会话。",
      "docs.apiRef": "API 参考",
      "docs.th.method": "方法",
      "docs.th.path": "路径",
      "docs.th.auth": "鉴权",
      "docs.th.desc": "说明",
      "docs.needApi": "需要调用 API？",
      "docs.needApiBody": "若你要基于 {{APP_NAME}} 构建应用，请切换到<a href=\"#\" class=\"js-docs-dev\">开发者指南</a>查看端点与示例。",
      "docs.crumb": "文档",
      "docs.crumbDevelopers": "开发者",
      "docs.crumbUsers": "用户",
      "docs.signOutHow": "退出登录",
      "docs.troubleshooting": "故障排查",
      "docs.youSee": "你看到",
      "docs.whatToDo": "处理方式",

      "toast.sessionExpired": "会话已过期 —— 请重新登录",
      "toast.signedOut": "已退出登录",
      "toast.newAvatar": "已生成新头像",
      "toast.avatarUploaded": "头像已上传",
      "toast.avatarFail": "无法刷新头像",
      "toast.avatarUploadFail": "头像上传失败",
      "toast.profileUpdated": "资料已更新",
      "toast.appsLoadFail": "加载应用失败",
      "toast.appNotFound": "未找到应用",
      "toast.editCancelled": "已取消编辑",
      "toast.uploadIconFirst": "请先创建应用，再上传图标",
      "toast.iconUploaded": "应用图标已上传",
      "toast.iconUploadFail": "图标上传失败",
      "toast.appUpdated": "应用已更新",
      "toast.appIdIssued": "已签发 App ID：{{id}}",
      "toast.appIdCopied": "App ID 已复制",
      "toast.secretRotated": "Secret 已轮换",
      "toast.rotateFail": "轮换失败",
      "toast.appRevoked": "应用已吊销",
      "toast.revokeFail": "吊销失败",
      "toast.secretCopied": "Secret 已复制",
      "toast.apiBaseCopied": "API 地址已复制",
      "toast.sessionRevoked": "会话已吊销 —— 请重新登录获取新令牌",
      "toast.sessionRevokeFail": "无法吊销会话",
      "toast.signInApps": "请登录后管理应用",
      "toast.switchAccount": "请使用其他账户登录",
      "toast.secretTitle": "请立即保存此 Secret",
      "toast.working": "处理中…",
      "err.emailInvalid": "邮箱格式无效",
      "err.usernameRequired": "请填写用户名",
      "err.passwordRequired": "请填写密码",
      "err.passwordMin": "密码至少 8 个字符",
      "err.passwordMatch": "两次输入的密码不一致",
      "err.appNameRequired": "请填写应用名称",
      "err.currentPassword": "请填写当前密码",
      "err.newPasswordMin": "新密码至少 8 个字符",
    },
  };

  const BRAND = (function () {
    const el = document.querySelector(".brand span");
    return (el && el.textContent.trim()) || "cloudflare-auth";
  })();

  function detectLang() {
    try {
      const saved = localStorage.getItem("cfa_lang");
      if (saved === "en" || saved === "zh") return saved;
    } catch (e) {}
    const nav = (navigator.language || "en").toLowerCase();
    return nav.indexOf("zh") === 0 ? "zh" : "en";
  }

  let lang = detectLang();

  function t(key, vars) {
    const table = DICT[lang] || DICT.en;
    let s = table[key];
    if (s == null) s = DICT.en[key];
    if (s == null) return key;
    s = s.replace(/\{\{APP_NAME\}\}/g, BRAND);
    if (vars) {
      Object.keys(vars).forEach(function (k) {
        s = s.replace(new RegExp("\\{\\{" + k + "\\}\\}", "g"), vars[k]);
      });
    }
    return s;
  }

  function applyDom() {
    document.querySelectorAll("[data-i18n]").forEach(function (el) {
      const key = el.getAttribute("data-i18n");
      const val = t(key);
      if (val && val !== key) el.textContent = val;
    });
    document.querySelectorAll("[data-i18n-html]").forEach(function (el) {
      const key = el.getAttribute("data-i18n-html");
      const val = t(key);
      if (val && val !== key) el.innerHTML = val;
    });
    document.querySelectorAll("[data-i18n-placeholder]").forEach(function (el) {
      const key = el.getAttribute("data-i18n-placeholder");
      const val = t(key);
      if (val && val !== key) el.setAttribute("placeholder", val);
    });
    document.querySelectorAll("[data-i18n-title]").forEach(function (el) {
      const key = el.getAttribute("data-i18n-title");
      const val = t(key);
      if (val && val !== key) el.setAttribute("title", val);
    });
    document.querySelectorAll("[data-lang]").forEach(function (btn) {
      btn.classList.toggle("on", btn.getAttribute("data-lang") === lang);
    });
    document.documentElement.lang = lang === "zh" ? "zh-CN" : "en";
    const title = t("meta.title");
    if (title) document.title = title;
  }

  function setLang(next) {
    if (next !== "en" && next !== "zh") return;
    lang = next;
    try {
      localStorage.setItem("cfa_lang", next);
    } catch (e) {}
    applyDom();
    if (typeof window.onI18nChange === "function") {
      try {
        window.onI18nChange(next);
      } catch (e) {}
    }
  }

  window.I18N = {
    get lang() {
      return lang;
    },
    t: t,
    setLang: setLang,
    apply: applyDom,
  };

  document.addEventListener("DOMContentLoaded", function () {
    applyDom();
    document.querySelectorAll("[data-lang]").forEach(function (btn) {
      btn.addEventListener("click", function (e) {
        e.preventDefault();
        setLang(btn.getAttribute("data-lang"));
      });
    });
  });
})();
