/* cloudflare-auth management console
 * Served from public/ via Workers assets binding.
 * Reads brand name from DOM (.brand span); no HTML placeholders here.
 * Keep UTF-8; avoid rewriting string literals with loose regex.
 */
    (function () {
      const API_BASE = "";
      const $ = (id) => document.getElementById(id);
      const views = {
        login: $("view-login"),
        register: $("view-register"),
        profile: $("view-profile"),
        apps: $("view-apps"),
        "authz-confirm": $("view-authz-confirm"),
        docs: $("view-docs"),
        "docs-dev": $("view-docs-dev"),
        "docs-user": $("view-docs-user"),
      };
      const sidebarFoot = $("sidebarFoot");
      const navAuth = $("navAuth");
      const navDocs = $("navDocs");
      const navApps = $("navApps");
      let appItems = [];
      let sessionExpiresIn = 86400;

      const store = {
        get token() { return localStorage.getItem("cfa_token"); },
        set token(v) { v ? localStorage.setItem("cfa_token", v) : localStorage.removeItem("cfa_token"); },
        get user() {
          try { return JSON.parse(localStorage.getItem("cfa_user") || "null"); }
          catch { return null; }
        },
        set user(v) { v ? localStorage.setItem("cfa_user", JSON.stringify(v)) : localStorage.removeItem("cfa_user"); },
      };

      /* ── API client & token store ── */
      function authHeaders() {
        const t = store.token;
        return t ? { Authorization: "Bearer " + t } : {};
      }

      async function api(path, options) {
        const opts = Object.assign({ headers: {} }, options || {});
        const headers = Object.assign(
          { "content-type": "application/json" },
          opts.headers,
          authHeaders()
        );
        const res = await fetch(API_BASE + path, Object.assign({}, opts, { headers }));
        let data = null;
        try { data = await res.json(); } catch { data = null; }
        if (!res.ok) {
          const err = new Error((data && data.message) || res.statusText || "Request failed");
          err.status = res.status;
          err.code = data && data.error;
          throw err;
        }
        return data;
      }

      /* ── Shell chrome / view routing ── */
      function setNav(active) {
        navAuth.classList.toggle("on", active === "auth");
        navDocs.classList.toggle("on", active === "docs");
        navApps.classList.toggle("on", active === "apps");
      }

      function setAuthChrome(signedIn) {
        navApps.hidden = !signedIn;
        sidebarFoot.hidden = true;
      }

      function show(name) {
        Object.entries(views).forEach(([k, el]) => el.classList.toggle("on", k === name));
        const signedIn = !!(store.token && store.user);
        setAuthChrome(signedIn);
        sidebarFoot.hidden = name !== "profile" && name !== "apps";
        const mainEl = document.querySelector(".main");
        if (mainEl) {
          mainEl.classList.toggle(
            "auth-mode",
            name === "login" || name === "register" || name === "authz-confirm"
          );
        }
        if (name === "docs" || name === "docs-dev" || name === "docs-user") {
          setNav("docs");
        } else if (name === "apps") {
          setNav("apps");
        } else if (name === "authz-confirm") {
          setNav("auth");
        } else {
          setNav("auth");
        }
        if (name !== "profile") {
          const panel = $("profileEditPanel");
          if (panel) panel.hidden = true;
        }
        window.scrollTo(0, 0);
      }

      function toast(msg) {
        const el = $("toast");
        el.textContent = msg;
        el.classList.add("on");
        clearTimeout(toast._t);
        toast._t = setTimeout(() => el.classList.remove("on"), 2500);
      }

      function setBanner(id, msg) {
        const el = $(id);
        el.textContent = msg || "";
        el.classList.toggle("on", !!msg);
      }

      function setFieldErr(inputId, errId, msg) {
        const input = $(inputId);
        const err = $(errId);
        input.setAttribute("aria-invalid", msg ? "true" : "false");
        err.textContent = msg || "";
      }

      function setLoading(btn, loading, label) {
        if (loading) {
          btn.disabled = true;
          btn.dataset.label = btn.textContent;
          btn.innerHTML = '<span class="spinner" aria-hidden="true"></span> ' + (label || "Working…");
        } else {
          btn.disabled = false;
          btn.textContent = btn.dataset.label || label || "Submit";
        }
      }

      /* ── Profile view ── */
      function fillProfile(user) {
        $("pfEmail").textContent = user.email;
        $("pfUsername").textContent = user.username;
        $("pfCreated").textContent = user.createdAt || user.created_at || "—";
        $("pfId").textContent = user.id;
        renderAvatar(user);
        const hours = Math.round((sessionExpiresIn || 86400) / 3600);
        const label = hours >= 24 ? Math.round(hours / 24) + "d" : hours + "h";
        const pill = document.querySelector("#view-profile .pill");
        if (pill) pill.innerHTML = '<span class="dot" aria-hidden="true"></span>Active · Expires in ' + label;
        const exp = new Date(Date.now() + (sessionExpiresIn || 86400) * 1000);
        $("pfExpiry").textContent = "This session will expire at " + exp.toISOString().replace(/\.\d{3}Z$/, "Z") + ".";
        const origin = location.origin;
        $("apiBase").textContent = origin;
      }

      function avatarSrc(user) {
        if (!user) return "";
        if (user.avatarUrl) {
          return user.avatarUrl.startsWith("http")
            ? user.avatarUrl
            : location.origin + user.avatarUrl;
        }
        return location.origin + "/users/" + encodeURIComponent(user.id) + "/avatar";
      }

      function renderAvatar(user) {
        const img = $("pfAvatar");
        const fb = $("pfAvatarFallback");
        if (!img || !fb) return;
        const name = (user && (user.username || user.email)) || "?";
        fb.textContent = String(name).charAt(0).toUpperCase();

        const src = avatarSrc(user);
        // 默认只显示字母回退，图片加载成功后再切换
        img.hidden = true;
        fb.hidden = false;
        img.removeAttribute("src");
        img.onload = null;
        img.onerror = null;

        if (!src) return;

        img.alt = name + " avatar";
        img.onload = () => {
          img.hidden = false;
          fb.hidden = true;
        };
        img.onerror = () => {
          img.hidden = true;
          fb.hidden = false;
        };
        img.src = src;
        // 缓存命中时 complete 可能已为 true，不会再次触发 onload
        if (img.complete && img.naturalWidth > 0) {
          img.hidden = false;
          fb.hidden = true;
        }
      }

      $("refreshAvatarBtn") && $("refreshAvatarBtn").addEventListener("click", async () => {
        try {
          const data = await api("/auth/avatar/refresh", { method: "POST" });
          store.user = data.user;
          renderAvatar(data.user);
          toast("New avatar generated");
        } catch (err) {
          if (err.status === 401) {
            clearSession();
            show("login");
            toast("Session expired — sign in again");
          } else {
            toast(err.message || "Could not refresh avatar");
          }
        }
      });

      /* —— Edit basic account info —— */
      function openProfileEdit() {
        const user = store.user || {};
        $("editEmail").value = user.email || "";
        $("editUsername").value = user.username || "";
        $("editCurrentPassword").value = "";
        $("editNewPassword").value = "";
        setBanner("profileEditBanner", "");
        ["editEmail|editEmailErr", "editUsername|editUsernameErr", "editNewPassword|editNewPasswordErr"]
          .forEach((pair) => {
            const [i, er] = pair.split("|");
            setFieldErr(i, er, "");
          });
        $("profileEditPanel").hidden = false;
        $("profileEditPanel").scrollIntoView({ behavior: "smooth", block: "nearest" });
        $("editEmail").focus();
      }

      function closeProfileEdit() {
        $("profileEditPanel").hidden = true;
        setBanner("profileEditBanner", "");
      }

      $("editProfileBtn") && $("editProfileBtn").addEventListener("click", () => {
        if ($("profileEditPanel").hidden) openProfileEdit();
        else closeProfileEdit();
      });
      $("profileEditCancel") && $("profileEditCancel").addEventListener("click", () => {
        closeProfileEdit();
      });

      $("profileEditForm") && $("profileEditForm").addEventListener("submit", async (e) => {
        e.preventDefault();
        setBanner("profileEditBanner", "");
        ["editEmail|editEmailErr", "editUsername|editUsernameErr", "editNewPassword|editNewPasswordErr"]
          .forEach((pair) => {
            const [i, er] = pair.split("|");
            setFieldErr(i, er, "");
          });

        const email = $("editEmail").value.trim().toLowerCase();
        const username = $("editUsername").value.trim();
        const currentPassword = $("editCurrentPassword").value;
        const newPassword = $("editNewPassword").value;

        let ok = true;
        if (!emailRe.test(email)) {
          setFieldErr("editEmail", "editEmailErr", "Email is invalid");
          ok = false;
        }
        if (!userRe.test(username)) {
          setFieldErr("editUsername", "editUsernameErr", "3–32 letters, numbers, underscore");
          ok = false;
        }
        if (newPassword && newPassword.length < 8) {
          setFieldErr("editNewPassword", "editNewPasswordErr", "Password must be at least 8 characters");
          ok = false;
        }
        if (newPassword && !currentPassword) {
          setBanner("profileEditBanner", "Enter current password to change password");
          ok = false;
        }
        if (!ok) return;

        const payload = { email, username };
        if (newPassword) {
          payload.currentPassword = currentPassword;
          payload.newPassword = newPassword;
        }

        const btn = $("profileEditSubmit");
        setLoading(btn, true, "Saving…");
        try {
          const data = await api("/auth/me", {
            method: "PATCH",
            body: JSON.stringify(payload),
          });
          store.user = data.user;
          fillProfile(data.user);
          closeProfileEdit();
          toast("Profile updated");
        } catch (err) {
          if (err.status === 401) {
            setBanner("profileEditBanner", err.message || "Current password is incorrect");
          } else if (err.status === 409) {
            setBanner("profileEditBanner", err.message || "Already taken");
          } else {
            setBanner("profileEditBanner", err.message || "Update failed");
          }
        } finally {
          setLoading(btn, false, "Save changes");
        }
      });

      function goProfile(user) {
        fillProfile(user);
        show("profile");
      }

      function clearSession() {
        store.token = null;
        store.user = null;
        appItems = [];
        hideSecretBanner();
      }

      async function copyText(text) {
        try {
          await navigator.clipboard.writeText(text);
          return true;
        } catch {
          const ta = document.createElement("textarea");
          ta.value = text;
          document.body.appendChild(ta);
          ta.select();
          const ok = document.execCommand("copy");
          ta.remove();
          return ok;
        }
      }

      async function flashCopied() {
        const flag = $("copiedFlag");
        flag.classList.add("on");
        clearTimeout(flashCopied._t);
        flashCopied._t = setTimeout(() => flag.classList.remove("on"), 1500);
      }

      const emailRe = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
      const userRe = /^[a-zA-Z0-9_]{3,32}$/;

      /* ── One-time secret banner ── */
      function showSecretOnce(title, message, secret) {
        $("secretBannerTitle").textContent = title;
        $("secretBannerMsg").textContent = message;
        $("secretValue").textContent = secret;
        $("secretBanner").classList.add("on");
      }

      function hideSecretBanner() {
        $("secretBanner").classList.remove("on");
      }

      /* —— Redirect login (authorize) context —— */
      const authz = {
        clientId: "",
        redirectUri: "",
        state: "",
        responseType: "code",
        appName: "",
        appDescription: "",
        appIcon: "",
        error: "",
      };

      /* ── OAuth authorize confirm flow ── */
      function parseAuthorizeQuery() {
        const q = new URLSearchParams(location.search);
        authz.clientId = (q.get("client_id") || q.get("app_id") || "").trim();
        authz.redirectUri = (q.get("redirect_uri") || "").trim();
        authz.state = q.get("state") || "";
        authz.responseType = (q.get("response_type") || "code").toLowerCase();
        authz.appName = "";
        authz.appDescription = "";
        authz.appIcon = "";
        authz.error = "";
      }

      function hasAuthzContext() {
        return !!(authz.clientId && authz.redirectUri);
      }

      function fillAuthzCard(ids, opts) {
        const box = $(ids.box);
        if (!box) return;
        const eyebrow = $(ids.eyebrow);
        const mark = $(ids.mark);
        const title = $(ids.title);
        const desc = $(ids.desc);
        const meta = $(ids.meta);

        if (opts.error) {
          box.classList.add("on", "err");
          if (eyebrow) eyebrow.textContent = "Authorization error";
          if (mark) mark.textContent = "!";
          if (title) title.textContent = "Sign-in unavailable";
          if (desc) desc.textContent = "";
          if (meta) meta.textContent = opts.error;
          return;
        }

        box.classList.add("on");
        box.classList.remove("err");
        const name = opts.name || "Application";
        if (eyebrow) eyebrow.textContent = "Requesting sign-in";
        if (mark) {
          if (opts.icon) {
            mark.innerHTML =
              '<img class="app-icon-img" alt="" width="36" height="36" src="' +
              escapeHtml(opts.icon) +
              '" />';
          } else {
            mark.textContent = String(name).charAt(0).toUpperCase() || "A";
          }
        }
        if (title) title.textContent = name;
        if (desc) {
          desc.textContent = opts.description || "";
          desc.hidden = !opts.description;
        }
        if (meta) {
          meta.innerHTML = opts.redirectUri
            ? "After sign-in, return to <code>" + escapeHtml(opts.redirectUri) + "</code>"
            : "";
        }
      }

      const brandEl = document.querySelector(".brand span");
      const UI_APP_NAME = (brandEl && brandEl.textContent.trim()) || "cloudflare-auth";

      function showAuthzBanners() {
        const shared = hasAuthzContext()
          ? {
              name: authz.appName,
              description: authz.appDescription,
              icon: authz.appIcon,
              redirectUri: authz.redirectUri,
              error: authz.error || "",
            }
          : null;

        const loginLede = $("loginLede");
        const regLede = $("regLede");

        if (!shared) {
          ["authzBanner", "authzBannerReg"].forEach((id) => {
            const box = $(id);
            if (box) box.classList.remove("on", "err");
          });
          if (loginLede) loginLede.textContent = "Use your " + UI_APP_NAME + " account";
          if (regLede) regLede.textContent = "Continue to " + UI_APP_NAME;
          return;
        }

        fillAuthzCard(
          {
            box: "authzBanner",
            eyebrow: "authzEyebrow",
            mark: "authzMark",
            title: "authzTitle",
            desc: "authzDesc",
            meta: "authzMeta",
          },
          shared
        );
        fillAuthzCard(
          {
            box: "authzBannerReg",
            eyebrow: "authzEyebrowReg",
            mark: "authzMarkReg",
            title: "authzTitleReg",
            desc: "authzDescReg",
            meta: "authzMetaReg",
          },
          shared
        );

        if (!shared.error && authz.appName) {
          if (loginLede) loginLede.textContent = "Sign in to continue to " + authz.appName;
          if (regLede) regLede.textContent = "Create an account for " + authz.appName;
        } else {
          if (loginLede) loginLede.textContent = "Use your " + UI_APP_NAME + " account";
          if (regLede) regLede.textContent = "Continue to " + UI_APP_NAME;
        }
      }

      async function loadAuthorizeContext() {
        parseAuthorizeQuery();
        if (!hasAuthzContext()) {
          showAuthzBanners();
          return false;
        }
        try {
          const data = await api(
            "/authorize?" +
              new URLSearchParams({
                client_id: authz.clientId,
                redirect_uri: authz.redirectUri,
                state: authz.state,
                response_type: authz.responseType,
              }).toString()
          );
          if (!data.ok) {
            authz.error = data.message || "Invalid authorize request";
            authz.appName = "";
            authz.appDescription = "";
            showAuthzBanners();
            return false;
          }
          authz.appName = (data.app && data.app.name) || "";
          authz.appDescription = (data.app && data.app.description) || "";
          authz.appIcon = (data.app && (data.app.iconUrl || data.app.iconPath)) || (authz.clientId ? "/v1/apps/" + encodeURIComponent(authz.clientId) + "/icon" : "");
          authz.appIcon = (data.app && (data.app.iconUrl || data.app.iconPath)) || (authz.clientId ? "/v1/apps/" + encodeURIComponent(authz.clientId) + "/icon" : "");
          authz.error = "";
          showAuthzBanners();
          return true;
        } catch (err) {
          authz.error = err.message || "Invalid authorize request";
          authz.appName = "";
          authz.appDescription = "";
          showAuthzBanners();
          return false;
        }
      }

      async function tryCompleteAuthorize() {
        if (!hasAuthzContext() || authz.error) return false;
        try {
          const data = await api("/auth/authorize", {
            method: "POST",
            body: JSON.stringify({
              client_id: authz.clientId,
              redirect_uri: authz.redirectUri,
              state: authz.state,
              response_type: authz.responseType,
            }),
          });
          if (data && data.redirectTo) {
            toast(
              authz.appName
                ? "Redirecting to " + authz.appName + "..."
                : "Redirecting..."
            );
            setTimeout(() => {
              window.location.assign(data.redirectTo);
            }, 300);
            return true;
          }
        } catch (err) {
          setBanner("loginBanner", err.message || "Authorization failed");
          setBanner("regBanner", err.message || "Authorization failed");
        }
        return false;
      }

      function renderAvatarInto(img, fb, user) {
        const name = (user && (user.username || user.email)) || "?";
        if (fb) {
          fb.textContent = String(name).charAt(0).toUpperCase();
          fb.hidden = false;
        }
        if (!img) return;
        const src = avatarSrc(user);
        img.hidden = true;
        img.alt = name + " avatar";
        img.onerror = () => {
          img.hidden = true;
          if (fb) fb.hidden = false;
        };
        img.onload = () => {
          img.hidden = false;
          if (fb) fb.hidden = true;
        };
        if (src) img.src = src;
      }

      function showAuthzConfirm(user) {
        const appName = authz.appName || "the application";
        const mark = $("authzConfirmMark");
        if (mark) {
          const iconSrc = authz.appIcon || "";
          if (iconSrc) {
            mark.innerHTML = '<img class="app-icon-img" alt="" width="40" height="40" src="' + escapeHtml(iconSrc) + '" />';
          } else {
            mark.textContent = String(appName).charAt(0).toUpperCase() || "A";
          }
        }
        const title = $("authzConfirmTitle");
        if (title) title.textContent = "Continue to " + appName + "?";
        const lede = $("authzConfirmLede");
        if (lede) {
          lede.textContent = authz.appDescription
            ? authz.appDescription
            : "You are already signed in. Confirm to share this account with " + appName + ".";
        }
        if ($("authzConfirmName")) {
          $("authzConfirmName").textContent = user.username || user.email || "—";
        }
        if ($("authzConfirmEmail")) {
          $("authzConfirmEmail").textContent = user.email || "";
        }
        const cont = $("authzContinueBtn");
        if (cont) cont.textContent = "Continue as " + (user.username || user.email || "this account");
        renderAvatarInto($("authzConfirmAvatar"), $("authzConfirmAvatarFb"), user);
        show("authz-confirm");
      }

      $("authzContinueBtn") && $("authzContinueBtn").addEventListener("click", async () => {
        const btn = $("authzContinueBtn");
        setLoading(btn, true, "Continuing…");
        try {
          const redirected = await tryCompleteAuthorize();
          if (!redirected) {
            // fall through to account console
            if (store.user) goProfile(store.user);
          }
        } finally {
          setLoading(btn, false, "Continue");
        }
      });

      $("authzSwitchBtn") && $("authzSwitchBtn").addEventListener("click", () => {
        // 保留 URL 上的 client_id / redirect_uri，仅清本地会话
        store.token = null;
        store.user = null;
        $("loginForm").reset();
        showAuthzBanners();
        show("login");
        toast("Sign in with another account");
      });

      $("authzSignOutBtn") && $("authzSignOutBtn").addEventListener("click", async () => {
        try {
          if (store.token) {
            await api("/auth/logout", { method: "POST" });
          }
        } catch {
          /* ignore */
        }
        clearSession();
        $("loginForm").reset();
        showAuthzBanners();
        show("login");
        toast("Signed out");
      });

      /* ── Applications CRUD ── */
      function parseRedirectUrisInput(raw) {
        return String(raw || "")
          .split(/[\n,]+/)
          .map((s) => s.trim())
          .filter(Boolean);
      }

      function appIconSrc(app) {
        if (app && app.iconUrl) return app.iconUrl;
        if (app && app.iconPath) return app.iconPath;
        if (app && app.appId) return "/v1/apps/" + encodeURIComponent(app.appId) + "/icon";
        return "";
      }

      function renderApps() {
        const list = $("appList");
        const empty = $("appsEmpty");
        const items = appItems;
        list.innerHTML = "";
        empty.hidden = items.length > 0;
        items.forEach((app) => {
          const uris = Array.isArray(app.redirectUris) ? app.redirectUris : [];
          const uriHtml = uris.length
            ? '<div class="app-id-line" style="margin-top:6px;"><span title="Redirect URIs">' +
              escapeHtml(uris.join(" · ")) +
              "</span></div>"
            : "";
          const iconSrc = appIconSrc(app);
          const initial = escapeHtml((app.name || "A").charAt(0).toUpperCase());
          const iconHtml = iconSrc
            ? '<img class="app-icon" alt="" width="40" height="40" src="' + escapeHtml(iconSrc) + '" />'
            : '<span class="app-icon fb">' + initial + '</span>';
          const row = document.createElement("div");
          row.className = "app-row" + (app.status === "revoked" ? " revoked" : "");
          row.innerHTML = `
            <div class="app-meta app-meta-with-icon">
              ${iconHtml}
              <div class="app-meta-text">
              <h3>${escapeHtml(app.name)}
                <span class="status-chip${app.status === "revoked" ? " revoked" : ""}">
                  <span class="dot" aria-hidden="true"></span>${app.status === "active" ? "Active" : "Revoked"}
                </span>
              </h3>
              <p>${escapeHtml(app.description || "No description")}</p>
              <div class="app-id-line">
                <span title="App ID">${escapeHtml(app.appId)}</span>
                <button type="button" data-copy-app="${escapeHtml(app.appId)}">Copy</button>
              </div>
              ${uriHtml}
              </div>
            </div>
            <div class="app-actions">
              <button type="button" class="btn btn-ghost btn-sm" data-edit="${escapeHtml(app.id)}">Edit</button>
              <button type="button" class="btn btn-ghost btn-sm" data-rotate="${escapeHtml(app.id)}" ${app.status !== "active" ? "disabled" : ""}>Rotate secret</button>
              <button type="button" class="btn btn-ghost btn-sm" data-revoke="${escapeHtml(app.id)}" ${app.status !== "active" ? "disabled" : ""}>Revoke</button>
            </div>
          `;
          list.appendChild(row);
        });
      }

      function escapeHtml(s) {
        return String(s)
          .replace(/&/g, "&amp;")
          .replace(/</g, "&lt;")
          .replace(/>/g, "&gt;")
          .replace(/"/g, "&quot;");
      }

      async function loadApps() {
        const data = await api("/apps");
        appItems = data.apps || [];
        renderApps();
      }

      async function openApps() {
        hideSecretBanner();
        resetAppFormMode();
        show("apps");
        try {
          await loadApps();
        } catch (err) {
          if (err.status === 401) {
            clearSession();
            show("login");
            toast("Session expired — sign in again");
            return;
          }
          toast(err.message || "Failed to load apps");
        }
      }

      /* —— App create / edit form mode —— */
      let editingAppId = null;

      function resetAppFormMode() {
        editingAppId = null;
        $("appFormTitle").textContent = "Register a new app";
        $("appFormHint").textContent =
          "Create an App ID, then register redirect URIs for redirect login.";
        $("appSubmit").textContent = "Create application";
        $("appCancelEdit").hidden = true;
        $("appIdReadonly").hidden = true;
        $("appEditAppId").textContent = "—";
        setBanner("appBanner", "");
        setFieldErr("appName", "appNameErr", "");
      }

      function startEditApp(id) {
        const app = appItems.find((x) => x.id === id);
        if (!app) {
          toast("App not found");
          return;
        }
        editingAppId = id;
        $("appFormTitle").textContent = "Edit application";
        $("appFormHint").textContent =
          "Update name, description, or redirect URIs. App Secret is not shown here.";
        $("appSubmit").textContent = "Save changes";
        $("appCancelEdit").hidden = false;
        $("appIdReadonly").hidden = false;
        $("appEditAppId").textContent = app.appId;
        $("appName").value = app.name || "";
        $("appDesc").value = app.description || "";
        $("appRedirectUris").value = (app.redirectUris || []).join("\n");
        if ($("appIconUrl")) $("appIconUrl").value = app.iconUrl || "";
        setBanner("appBanner", "");
        setFieldErr("appName", "appNameErr", "");
        $("appForm").scrollIntoView({ behavior: "smooth", block: "start" });
        $("appName").focus();
      }

      $("appCancelEdit").addEventListener("click", () => {
        $("appForm").reset();
        resetAppFormMode();
        toast("Edit cancelled");
      });

      $("appForm").addEventListener("submit", async (e) => {
        e.preventDefault();
        setBanner("appBanner", "");
        setFieldErr("appName", "appNameErr", "");
        const name = $("appName").value.trim();
        if (!name) {
          setFieldErr("appName", "appNameErr", "App name is required");
          return;
        }
        const payload = {
          name,
          description: $("appDesc").value.trim(),
          redirectUris: parseRedirectUrisInput($("appRedirectUris").value),
          iconUrl: ($("appIconUrl") && $("appIconUrl").value.trim()) || null,
        };
        const btn = $("appSubmit");
        const isEdit = !!editingAppId;
        setLoading(btn, true, isEdit ? "Saving…" : "Creating…");
        try {
          if (isEdit) {
            const data = await api("/apps/" + encodeURIComponent(editingAppId), {
              method: "PUT",
              body: JSON.stringify(payload),
            });
            await loadApps();
            $("appForm").reset();
            resetAppFormMode();
            toast("Application updated");
            return;
          }

          const data = await api("/apps", {
            method: "POST",
            body: JSON.stringify(payload),
          });
          $("appForm").reset();
          resetAppFormMode();
          await loadApps();
          showSecretOnce(
            "Application created",
            data.warning || "Copy this App Secret now. It will not be shown again.",
            data.appSecret
          );
          toast("App ID issued: " + data.app.appId);
        } catch (err) {
          if (err.status === 401) {
            clearSession();
            show("login");
            toast("Session expired — sign in again");
          } else {
            setBanner("appBanner", err.message || (isEdit ? "Update failed" : "Create failed"));
          }
        } finally {
          setLoading(btn, false, isEdit ? "Save changes" : "Create application");
        }
      });

      $("appList").addEventListener("click", async (e) => {
        const t = e.target;
        if (!(t instanceof HTMLElement)) return;
        const copyAppId = t.getAttribute("data-copy-app");
        if (copyAppId) {
          if (await copyText(copyAppId)) toast("App ID copied");
          return;
        }
        const editId = t.getAttribute("data-edit");
        if (editId) {
          startEditApp(editId);
          return;
        }
        const rotateId = t.getAttribute("data-rotate");
        if (rotateId) {
          try {
            const data = await api("/apps/" + encodeURIComponent(rotateId) + "/rotate-secret", { method: "POST" });
            await loadApps();
            showSecretOnce(
              "Secret rotated",
              data.warning || "The previous secret is invalid. Store the new one now.",
              data.appSecret
            );
            toast("Secret rotated");
          } catch (err) {
            if (err.status === 401) {
              clearSession();
              show("login");
              toast("Session expired — sign in again");
            } else {
              toast(err.message || "Rotate failed");
            }
          }
          return;
        }
        const revokeId = t.getAttribute("data-revoke");
        if (revokeId) {
          if (!confirm("Revoke this application? Integrations using its App ID will stop working.")) return;
          try {
            await api("/apps/" + encodeURIComponent(revokeId) + "/revoke", { method: "POST" });
            hideSecretBanner();
            if (editingAppId === revokeId) {
              $("appForm").reset();
              resetAppFormMode();
            }
            await loadApps();
            toast("Application revoked");
          } catch (err) {
            if (err.status === 401) {
              clearSession();
              show("login");
              toast("Session expired — sign in again");
            } else {
              toast(err.message || "Revoke failed");
            }
          }
        }
      });

      $("copySecretBtn").addEventListener("click", async () => {
        if (await copyText($("secretValue").textContent.trim())) toast("Secret copied");
      });

      $("gotoAppsBtn").addEventListener("click", () => {
        openApps();
      });

      $("navApps").addEventListener("click", (e) => {
        e.preventDefault();
        if (!(store.token && store.user)) {
          show("login");
          return;
        }
        openApps();
      });

      /* 鈥斺€?Login form 鈥斺€?*/
      /* ── Sign-in / Sign-up forms ── */
      $("loginForm").addEventListener("submit", async (e) => {
        e.preventDefault();
        setBanner("loginBanner", "");
        setFieldErr("loginEmail", "loginEmailErr", "");
        setFieldErr("loginPassword", "loginPasswordErr", "");

        const email = $("loginEmail").value.trim().toLowerCase();
        const password = $("loginPassword").value;
        let ok = true;
        if (!emailRe.test(email)) {
          setFieldErr("loginEmail", "loginEmailErr", "Email is invalid");
          ok = false;
        }
        if (!password) {
          setFieldErr("loginPassword", "loginPasswordErr", "Password is required");
          ok = false;
        }
        if (!ok) return;

        const btn = $("loginSubmit");
        setLoading(btn, true, "Signing in…");
        try {
          const res = await api("/auth/login", {
            method: "POST",
            body: JSON.stringify({ email, password }),
          });
          sessionExpiresIn = res.expiresIn || 86400;
          store.token = res.token;
          store.user = res.user;
          if (hasAuthzContext()) {
            const redirected = await tryCompleteAuthorize();
            if (redirected) {
              setLoading(btn, false, "Sign in");
              return;
            }
          }
          goProfile(res.user);
        } catch (err) {
          setBanner("loginBanner", err.message || "Sign in failed");
          $("loginPassword").focus();
        } finally {
          setLoading(btn, false, "Sign in");
        }
      });

      /* 鈥斺€?Register form 鈥斺€?*/
      $("registerForm").addEventListener("submit", async (e) => {
        e.preventDefault();
        setBanner("regBanner", "");
        ["regEmail|regEmailErr", "regUsername|regUsernameErr", "regPassword|regPasswordErr", "regConfirm|regConfirmErr"]
          .forEach((pair) => {
            const [i, er] = pair.split("|");
            setFieldErr(i, er, "");
          });

        const email = $("regEmail").value.trim().toLowerCase();
        const username = $("regUsername").value.trim();
        const password = $("regPassword").value;
        const confirm = $("regConfirm").value;
        let ok = true;
        if (!emailRe.test(email)) { setFieldErr("regEmail", "regEmailErr", "Email is invalid"); ok = false; }
        if (!userRe.test(username)) { setFieldErr("regUsername", "regUsernameErr", "3–32 letters, numbers, underscore"); ok = false; }
        if (password.length < 8) { setFieldErr("regPassword", "regPasswordErr", "Password must be at least 8 characters"); ok = false; }
        if (confirm !== password) { setFieldErr("regConfirm", "regConfirmErr", "Passwords do not match"); ok = false; }
        if (!ok) return;

        const btn = $("regSubmit");
        setLoading(btn, true, "Creating…");
        try {
          const res = await api("/auth/register", {
            method: "POST",
            body: JSON.stringify({ email, username, password }),
          });
          sessionExpiresIn = res.expiresIn || 86400;
          store.token = res.token;
          store.user = res.user;
          if (hasAuthzContext()) {
            const redirected = await tryCompleteAuthorize();
            if (redirected) {
              setLoading(btn, false, "Create account");
              return;
            }
          }
          goProfile(res.user);
        } catch (err) {
          setBanner("regBanner", err.message || "Registration failed");
        } finally {
          setLoading(btn, false, "Create account");
        }
      });

      $("toRegister").addEventListener("click", (e) => {
        e.preventDefault();
        setBanner("loginBanner", "");
        const email = $("loginEmail").value.trim();
        if (email) $("regEmail").value = email;
        show("register");
      });
      $("toLogin").addEventListener("click", (e) => {
        e.preventDefault();
        setBanner("regBanner", "");
        show("login");
      });

      $("signOutBtn").addEventListener("click", async () => {
        try {
          if (store.token) await api("/auth/logout", { method: "POST" });
        } catch { /* ignore */ }
        clearSession();
        $("loginForm").reset();
        show("login");
        toast("Signed out");
      });

      $("copyIconBtn").addEventListener("click", async () => {
        if (await copyText($("apiBase").textContent.trim())) flashCopied();
      });
      $("copyApiBtn").addEventListener("click", async () => {
        if (await copyText($("apiBase").textContent.trim())) {
          flashCopied();
          toast("API Base URL copied");
        }
      });
      $("regenBtn").addEventListener("click", async () => {
        try {
          await api("/auth/logout", { method: "POST" });
          clearSession();
          show("login");
          toast("Session revoked — sign in for a new token");
        } catch (err) {
          toast(err.message || "Could not revoke session");
        }
      });

      $("brandLink").addEventListener("click", (e) => {
        e.preventDefault();
        if (store.token && store.user) goProfile(store.user);
        else show("login");
      });

      /* 鈥斺€?Docs navigation 鈥斺€?*/
      navAuth.addEventListener("click", (e) => {
        e.preventDefault();
        if (store.token && store.user) goProfile(store.user);
        else show("login");
      });
      navDocs.addEventListener("click", (e) => {
        e.preventDefault();
        show("docs");
      });
      $("cardDocsDev").addEventListener("click", (e) => {
        e.preventDefault();
        show("docs-dev");
      });
      $("cardDocsUser").addEventListener("click", (e) => {
        e.preventDefault();
        show("docs-user");
      });
      document.querySelectorAll(".js-docs").forEach((el) => {
        el.addEventListener("click", (e) => {
          e.preventDefault();
          show("docs");
        });
      });
      document.querySelectorAll(".js-docs-dev").forEach((el) => {
        el.addEventListener("click", (e) => {
          e.preventDefault();
          show("docs-dev");
        });
      });
      document.querySelectorAll(".js-docs-user").forEach((el) => {
        el.addEventListener("click", (e) => {
          e.preventDefault();
          show("docs-user");
        });
      });
      document.querySelectorAll(".js-apps").forEach((el) => {
        el.addEventListener("click", (e) => {
          e.preventDefault();
          if (store.token && store.user) {
            openApps();
          } else {
            show("login");
            toast("Sign in to manage applications");
          }
        });
      });

      /* Boot: restore session via /auth/me */
      /* ── Boot ── */
      async function boot() {
        $("apiBase").textContent = location.origin;
        await loadAuthorizeContext();
        if (!store.token) {
          show("login");
          showAuthzBanners();
          return;
        }
        try {
          const data = await api("/auth/me");
          store.user = data.user;
          if (hasAuthzContext() && !authz.error) {
            // 已登录 + 第三方跳转：二次确认，不自动授权
            showAuthzConfirm(data.user);
            return;
          }
          goProfile(data.user);
        } catch {
          clearSession();
          show("login");
          showAuthzBanners();
        }
      }
      boot();
    })();
  
