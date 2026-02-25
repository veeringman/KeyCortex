const state = {
  manifest: null,
  wallets: [],        // [{wallet_address, chain, bound_user_id, public_key, label}]
  profiles: [],       // [{id, name}]
  activeProfile: null, // profile id
  activeWallet: null,  // wallet_address
};

const byId = (id) => document.getElementById(id);

const elements = {
  walletWindow: document.querySelector(".wallet-window"),
  walletFolded: document.getElementById("walletFolded"),
  walletFoldToggle: document.getElementById("walletFoldToggle"),
  walletApp: document.getElementById("walletApp"),
  baseUrl: byId("baseUrl"),
  skinSelect: byId("skinSelect"),
  skinCycleBtn: byId("skinCycleBtn"),
  tabs: Array.from(document.querySelectorAll(".tab")),
  panels: Array.from(document.querySelectorAll(".panel")),
  createWalletBtn: byId("createWalletBtn"),
  createResult: byId("createResult"),
  connectWalletAddress: byId("connectWalletAddress"),
  connectChain: byId("connectChain"),
  connectToken: byId("connectToken"),
  bindWalletBtn: byId("bindWalletBtn"),
  connectResult: byId("connectResult"),
  balanceWalletAddress: byId("balanceWalletAddress"),
  balanceChain: byId("balanceChain"),
  balanceAsset: byId("balanceAsset"),
  balanceNetworkIcon: byId("balanceNetworkIcon"),
  balanceCoinIcon: byId("balanceCoinIcon"),
  balanceBtn: byId("balanceBtn"),
  balanceResult: byId("balanceResult"),
  signWalletAddress: byId("signWalletAddress"),
  signPurpose: byId("signPurpose"),
  signPayload: byId("signPayload"),
  signBtn: byId("signBtn"),
  signResult: byId("signResult"),
  challengeBtn: byId("challengeBtn"),
  verifyBtn: byId("verifyBtn"),
  submitFrom: byId("submitFrom"),
  submitTo: byId("submitTo"),
  submitAmount: byId("submitAmount"),
  submitAsset: byId("submitAsset"),
  submitChain: byId("submitChain"),
  submitNonce: byId("submitNonce"),
  nonceBtn: byId("nonceBtn"),
  nonceDisplay: byId("nonceDisplay"),
  submitTxBtn: byId("submitTxBtn"),
  submitResult: byId("submitResult"),
  txHash: byId("txHash"),
  txStatusBtn: byId("txStatusBtn"),
  historyResult: byId("historyResult"),
  // Wallets tab
  walletListContainer: byId("walletListContainer"),
  refreshWalletsBtn: byId("refreshWalletsBtn"),
  walletLabelInput: byId("walletLabelInput"),
  walletPassphraseInput: byId("walletPassphraseInput"),
  restoreWalletBtn: byId("restoreWalletBtn"),
  restoreHint: byId("restoreHint"),
  halfFoldWalletName: byId("halfFoldWalletName"),
  halfFoldChain: byId("halfFoldChain"),
  // Profile & wallet selector
  profileSelect: byId("profileSelect"),
  addProfileBtn: byId("addProfileBtn"),
  activeWalletSelect: byId("activeWalletSelect"),
  // Platform integration elements
  chainConfigBtn: byId("chainConfigBtn"),
  chainConfigResult: byId("chainConfigResult"),
  fdWalletAddress: byId("fdWalletAddress"),
  walletStatusBtn: byId("walletStatusBtn"),
  walletStatusResult: byId("walletStatusResult"),
  pcWalletAddress: byId("pcWalletAddress"),
  pcChallenge: byId("pcChallenge"),
  pcTxHash: byId("pcTxHash"),
  commitmentBtn: byId("commitmentBtn"),
  commitmentResult: byId("commitmentResult"),
  healthBtn: byId("healthBtn"),
  readyzBtn: byId("readyzBtn"),
  startupzBtn: byId("startupzBtn"),
  opsResult: byId("opsResult"),
};


// Wallet fold states: "folded" → "half" → "unfolded"
let walletFoldState = "folded"; // "folded" | "half" | "unfolded"
let autoFoldTimer = null;
let autoCloseTimer = null;

function resetAutoFoldTimer() {
  if (autoFoldTimer) clearTimeout(autoFoldTimer);
  if (autoCloseTimer) clearTimeout(autoCloseTimer);
  autoFoldTimer = null;
  autoCloseTimer = null;
  if (walletFoldState === "unfolded") {
    // Half-fold after 30s of inactivity
    autoFoldTimer = setTimeout(() => {
      if (walletFoldState === "unfolded") {
        setWalletState("half");
      }
    }, 30000);
    // Fully close after 2 minutes of inactivity
    autoCloseTimer = setTimeout(() => {
      if (walletFoldState !== "folded") {
        setWalletState("folded");
      }
    }, 120000);
  } else if (walletFoldState === "half") {
    // If already half-folded, fully close after 90s more
    autoCloseTimer = setTimeout(() => {
      if (walletFoldState === "half") {
        setWalletState("folded");
      }
    }, 90000);
  }
}

function setWalletState(newState) {
  walletFoldState = newState;
  const win = elements.walletWindow;
  const overlay = elements.walletFolded;
  const app = elements.walletApp;

  // Remove all state classes
  win.classList.remove("folded", "half-folded", "unfolded");
  overlay.classList.remove("closed", "half", "open");

  if (newState === "folded") {
    win.classList.add("folded");
    overlay.classList.add("closed");
    app.style.visibility = "hidden";
    app.style.opacity = "0";
    app.style.pointerEvents = "none";
  } else if (newState === "half") {
    win.classList.add("half-folded");
    overlay.classList.add("half");
    app.style.visibility = "visible";
    app.style.opacity = "1";
    app.style.pointerEvents = "auto";
    // Size shutter to cover only header, not the network/content below
    const upperHalf = app.querySelector(".wallet-upper-half");
    const flap = win.querySelector(".wallet-flap");
    if (upperHalf && flap) {
      const h = upperHalf.offsetHeight + flap.offsetHeight;
      overlay.style.setProperty("--half-fold-height", h + "px");
    }
  } else {
    // unfolded
    win.classList.add("unfolded");
    overlay.classList.add("open");
    app.style.visibility = "visible";
    app.style.opacity = "1";
    app.style.pointerEvents = "auto";
  }
  resetAutoFoldTimer();
}

// Initial state: folded

document.addEventListener("DOMContentLoaded", () => {
  setWalletState("folded");

  // Click overlay logo: folded → half → unfolded
  elements.walletFoldToggle.addEventListener("click", (e) => {
    e.stopPropagation();
    if (walletFoldState === "folded") {
      setWalletState("half");
    } else if (walletFoldState === "half") {
      setWalletState("unfolded");
    }
  });

  elements.walletFolded.addEventListener("click", (e) => {
    if (e.target === elements.walletFolded || e.target === elements.walletFoldToggle) return;
    if (walletFoldState === "folded") {
      setWalletState("half");
    } else if (walletFoldState === "half") {
      setWalletState("unfolded");
    }
  });

  // Click brand logo in header: unfolded → half-fold
  const brandLogo = document.querySelector(".brand-logo");
  if (brandLogo) {
    brandLogo.style.cursor = "pointer";
    brandLogo.addEventListener("click", (e) => {
      e.stopPropagation();
      if (walletFoldState === "unfolded") {
        setWalletState("half");
      }
    });
  }

  // Reset auto-fold timer on user interaction within wallet
  elements.walletApp.addEventListener("click", () => resetAutoFoldTimer());
  elements.walletApp.addEventListener("input", () => resetAutoFoldTimer());
  elements.walletApp.addEventListener("focus", () => resetAutoFoldTimer(), true);
});

function baseUrl() {
  const v = elements.baseUrl.value.trim().replace(/\/+$/, "");
  if (v) return v;
  // Auto-detect: same origin but port 8080; handles Codespace/devcontainer forwarding
  const loc = window.location;
  if (loc.hostname.includes("app.github.dev") || loc.hostname.includes("preview.app.github.dev")) {
    // Codespace: replace port portion in hostname  e.g. xxx-8090.app.github.dev → xxx-8080.app.github.dev
    return loc.protocol + "//" + loc.hostname.replace(/\d+-(\d+)/, (m, p) => m.replace(p, "8080")).replace("-8090.", "-8080.");
  }
  return loc.protocol + "//" + loc.hostname + ":8080";
}


let themes = null;

async function loadThemes() {
  if (themes) return themes;
  const resp = await fetch("./themes.json");
  if (!resp.ok) throw new Error("Failed to load themes.json");
  themes = await resp.json();
  return themes;
}

function setThemeVars(theme) {
  const root = elements.walletWindow;
  root.style.setProperty('--wallet-skin', `url('${theme.backgroundPattern}')`);
  root.style.setProperty('--wallet-skin-size', '12px 12px');
  root.style.setProperty('--edge-color', theme.edge);
  root.style.setProperty('--edge-color-soft', theme.secondary);
  root.style.setProperty('--glass-bg', theme.glass);
  root.style.setProperty('--wallet-text', theme.text);
  root.style.setProperty('--wallet-accent', theme.accent);
  // Stitch, clip, and checkered theme vars
  if (theme.stitchColor) root.style.setProperty('--stitch-color', theme.stitchColor);
  if (theme.clipHighlight) root.style.setProperty('--clip-highlight', theme.clipHighlight);
  if (theme.clipColor) root.style.setProperty('--clip-color', theme.clipColor);
  if (theme.clipShadow) root.style.setProperty('--clip-shadow', theme.clipShadow);
  if (theme.checkeredOpacity) root.style.setProperty('--checkered-opacity', theme.checkeredOpacity);
  // Muted text color for labels/hints
  const mutedMap = { classic: '#64748b', dark: '#b8a080', black: '#999', navy: '#8ba4c8', forest: '#8caa7a' };
  const skinKey = elements.skinSelect.value || 'classic';
  root.style.setProperty('--wallet-text-muted', mutedMap[skinKey] || '#64748b');
}

async function applySkin(skin) {
  const root = elements.walletWindow;
  root.classList.remove("skin-dark", "skin-black", "skin-navy", "skin-forest");
  // Load theme tokens
  const allThemes = themes || await loadThemes();
  const theme = allThemes[skin] || allThemes.classic;
  setThemeVars(theme);
  if (skin !== "classic") {
    root.classList.add(`skin-${skin}`);
  }
}

function cycleSkin() {
  const skins = ["classic", "dark", "black", "navy", "forest"];
  const current = elements.skinSelect.value;
  const index = skins.indexOf(current);
  const next = skins[(index + 1) % skins.length];
  elements.skinSelect.value = next;
  applySkin(next);
  localStorage.setItem("kc_wallet_skin", next);
}

function setResult(target, payload, isError = false) {
  target.textContent =
    typeof payload === "string" ? payload : JSON.stringify(payload, null, 2);
  target.style.borderColor = isError ? "#ef4444" : "#d8dee8";
}

function setActiveTab(tabId) {
  for (const tab of elements.tabs) {
    tab.classList.toggle("active", tab.dataset.tab === tabId);
  }
  for (const panel of elements.panels) {
    panel.classList.toggle("active", panel.id === tabId);
  }
}

async function request(path, options = {}) {
  const response = await fetch(`${baseUrl()}${path}`, {
    headers: {
      "content-type": "application/json",
      ...(options.headers || {}),
    },
    ...options,
  });

  const contentType = response.headers.get("content-type") || "";
  const body = contentType.includes("application/json")
    ? await response.json()
    : await response.text();

  if (!response.ok) {
    throw new Error(
      typeof body === "string" ? body : body?.error || JSON.stringify(body),
    );
  }

  return body;
}

function toBase64(input) {
  return btoa(unescape(encodeURIComponent(input)));
}

function resolveNetworkIcon(chain) {
  const manifest = state.manifest;
  if (!manifest) {
    return "../assets/icons/common/network-default.svg";
  }
  return manifest.networks?.[chain]?.icon?.replace(/^ui\//, "../") || manifest.defaults.network.replace(/^ui\//, "../");
}

function normalizeAsset(asset) {
  if (asset.toLowerCase() === "flower") {
    return "FloweR";
  }
  if (asset === "FloweR") {
    return "FloweR";
  }
  return asset.toUpperCase();
}

function resolveCoinIcon(asset) {
  const manifest = state.manifest;
  if (!manifest) {
    return "../assets/icons/common/coin-default.svg";
  }
  const key = normalizeAsset(asset);
  return manifest.coins?.[key]?.icon?.replace(/^ui\//, "../") || manifest.defaults.coin.replace(/^ui\//, "../");
}

function updateBalanceIcons() {
  const chain = elements.balanceChain.value.trim() || "flowcortex-l1";
  const asset = elements.balanceAsset.value;
  elements.balanceNetworkIcon.src = resolveNetworkIcon(chain);
  elements.balanceCoinIcon.src = resolveCoinIcon(asset);
}

async function loadManifest() {
  try {
    const response = await fetch("../config/icon-manifest.json");
    if (!response.ok) {
      return;
    }
    state.manifest = await response.json();
  } catch {
    state.manifest = null;
  } finally {
    updateBalanceIcons();
  }
}

async function onCreateWallet() {
  try {
    const label = elements.walletLabelInput.value.trim() || undefined;
    const passphrase = elements.walletPassphraseInput.value || undefined;
    const body = {};
    if (label) body.label = label;
    if (passphrase) body.passphrase = passphrase;
    const result = await request("/wallet/create", {
      method: "POST",
      body: JSON.stringify(body),
    });
    setResult(elements.createResult, result);
    // Clear inputs
    elements.walletLabelInput.value = "";
    elements.walletPassphraseInput.value = "";
    // Refresh wallet list and select new wallet
    await loadWalletList();
    selectActiveWallet(result.wallet_address);
    elements.createResult.scrollIntoView({ behavior: "smooth", block: "nearest" });
  } catch (error) {
    setResult(elements.createResult, error.message, true);
  }
}

async function onRestoreWallet() {
  try {
    const passphrase = elements.walletPassphraseInput.value;
    if (!passphrase) throw new Error("passphrase is required for restore");
    const label = elements.walletLabelInput.value.trim() || undefined;
    const body = { passphrase };
    if (label) body.label = label;
    const result = await request("/wallet/restore", {
      method: "POST",
      body: JSON.stringify(body),
    });
    const msg = result.already_existed
      ? `✓ Restored (already existed): ${result.wallet_address.slice(0,10)}…`
      : `✓ Restored (new): ${result.wallet_address.slice(0,10)}…`;
    elements.restoreHint.textContent = msg;
    elements.restoreHint.style.display = "block";
    elements.restoreHint.style.color = "var(--wallet-accent, #059669)";
    setResult(elements.createResult, result);
    elements.walletLabelInput.value = "";
    elements.walletPassphraseInput.value = "";
    await loadWalletList();
    selectActiveWallet(result.wallet_address);
    elements.createResult.scrollIntoView({ behavior: "smooth", block: "nearest" });
    setTimeout(() => { elements.restoreHint.style.display = "none"; }, 5000);
  } catch (error) {
    elements.restoreHint.textContent = error.message;
    elements.restoreHint.style.display = "block";
    elements.restoreHint.style.color = "#ef4444";
    setTimeout(() => { elements.restoreHint.style.display = "none"; }, 5000);
  }
}

async function onRenameWallet(walletAddress) {
  const w = state.wallets.find(w => w.wallet_address === walletAddress);
  const currentName = w?.label || "";
  const newName = prompt("Rename wallet:", currentName);
  if (newName === null || !newName.trim()) return;
  try {
    await request("/wallet/rename", {
      method: "POST",
      body: JSON.stringify({ wallet_address: walletAddress, label: newName.trim() }),
    });
    await loadWalletList();
  } catch (error) {
    alert("Rename failed: " + error.message);
  }
}

async function onBindWallet() {
  try {
    const token = elements.connectToken.value.trim();
    const result = await request("/auth/bind", {
      method: "POST",
      headers: token ? { authorization: `Bearer ${token}` } : {},
      body: JSON.stringify({
        wallet_address: elements.connectWalletAddress.value.trim(),
        chain: elements.connectChain.value.trim() || "flowcortex-l1",
      }),
    });
    setResult(elements.connectResult, result);
  } catch (error) {
    setResult(elements.connectResult, error.message, true);
  }
}

async function onFetchBalance() {
  try {
    const query = new URLSearchParams({
      wallet_address: elements.balanceWalletAddress.value.trim(),
      chain: elements.balanceChain.value.trim() || "flowcortex-l1",
      asset: elements.balanceAsset.value,
    });

    const result = await request(`/wallet/balance?${query.toString()}`, {
      method: "GET",
      headers: {},
    });
    setResult(elements.balanceResult, result);
  } catch (error) {
    setResult(elements.balanceResult, error.message, true);
  }
}

async function onSignPayload() {
  try {
    const result = await request("/wallet/sign", {
      method: "POST",
      body: JSON.stringify({
        wallet_address: elements.signWalletAddress.value.trim(),
        payload: toBase64(elements.signPayload.value),
        purpose: elements.signPurpose.value,
      }),
    });
    setResult(elements.signResult, result);
  } catch (error) {
    setResult(elements.signResult, error.message, true);
  }
}

async function onFetchTxStatus() {
  try {
    const txHash = elements.txHash.value.trim();
    const result = await request(`/wallet/tx/${encodeURIComponent(txHash)}`, {
      method: "GET",
      headers: {},
    });
    setResult(elements.historyResult, result);
  } catch (error) {
    setResult(elements.historyResult, error.message, true);
  }
}

// Track last challenge for auto-verify flow
let lastChallenge = null;

async function onChallenge() {
  try {
    const result = await request("/auth/challenge", { method: "POST" });
    lastChallenge = result.challenge;
    setResult(elements.connectResult, result);
  } catch (error) {
    setResult(elements.connectResult, error.message, true);
  }
}

async function onVerify() {
  try {
    const walletAddress = elements.connectWalletAddress.value.trim();
    if (!walletAddress) throw new Error("wallet address required");
    const challenge = lastChallenge;
    if (!challenge) throw new Error("request a challenge first");

    // Sign the challenge with the wallet
    const signResult = await request("/wallet/sign", {
      method: "POST",
      body: JSON.stringify({
        wallet_address: walletAddress,
        payload: toBase64(challenge),
        purpose: "auth",
      }),
    });

    // Verify the signature
    const result = await request("/auth/verify", {
      method: "POST",
      body: JSON.stringify({
        wallet_address: walletAddress,
        signature: signResult.signature,
        challenge: challenge,
      }),
    });
    setResult(elements.connectResult, result);
  } catch (error) {
    setResult(elements.connectResult, error.message, true);
  }
}

async function onFetchNonce() {
  try {
    const walletAddress = elements.submitFrom.value.trim();
    if (!walletAddress) throw new Error("set 'From' address first");
    const query = new URLSearchParams({ wallet_address: walletAddress });
    const result = await request(`/wallet/nonce?${query.toString()}`, {
      method: "GET",
      headers: {},
    });
    elements.nonceDisplay.textContent = `last: ${result.last_nonce} · next: ${result.next_nonce}`;
    elements.submitNonce.value = result.next_nonce;
  } catch (error) {
    elements.nonceDisplay.textContent = "—";
    setResult(elements.submitResult, error.message, true);
  }
}

async function onSubmitTx() {
  try {
    const nonce = parseInt(elements.submitNonce.value, 10);
    if (!nonce || nonce < 1) throw new Error("nonce required (use Get Nonce)");
    const result = await request("/wallet/submit", {
      method: "POST",
      body: JSON.stringify({
        from: elements.submitFrom.value.trim(),
        to: elements.submitTo.value.trim(),
        amount: elements.submitAmount.value.trim(),
        asset: elements.submitAsset.value,
        chain: elements.submitChain.value.trim() || "flowcortex-l1",
        nonce: nonce,
      }),
    });
    setResult(elements.submitResult, result);
    // Populate tx hash for easy lookup
    if (result.tx_hash) {
      elements.txHash.value = result.tx_hash;
    }
  } catch (error) {
    setResult(elements.submitResult, error.message, true);
  }
}

// --- Platform Integration Handlers ---

// ── Profile management (local) ──
function loadProfiles() {
  try {
    state.profiles = JSON.parse(localStorage.getItem("kc_profiles") || "[]");
  } catch { state.profiles = []; }
  if (state.profiles.length === 0) {
    state.profiles = [{ id: "default", name: "Default User" }];
    saveProfiles();
  }
  state.activeProfile = localStorage.getItem("kc_active_profile") || state.profiles[0].id;
  renderProfileSelect();
}

function saveProfiles() {
  localStorage.setItem("kc_profiles", JSON.stringify(state.profiles));
}

function renderProfileSelect() {
  const sel = elements.profileSelect;
  sel.innerHTML = "";
  for (const p of state.profiles) {
    const opt = document.createElement("option");
    opt.value = p.id;
    opt.textContent = p.name;
    if (p.id === state.activeProfile) opt.selected = true;
    sel.appendChild(opt);
  }
}

async function onProfileChange() {
  state.activeProfile = elements.profileSelect.value;
  localStorage.setItem("kc_active_profile", state.activeProfile);
  await loadWalletList();
  updateHalfFoldInfo();
}

function onAddProfile() {
  const name = prompt("Enter profile / user name:");
  if (!name || !name.trim()) return;
  const id = "profile-" + Date.now();
  state.profiles.push({ id, name: name.trim() });
  saveProfiles();
  state.activeProfile = id;
  localStorage.setItem("kc_active_profile", id);
  renderProfileSelect();
  loadWalletList();
  updateHalfFoldInfo();
}

// ── Wallet list ──
async function loadWalletList() {
  try {
    const result = await request("/wallet/list", { method: "GET", headers: {} });
    state.wallets = result.wallets || [];
  } catch {
    state.wallets = [];
  }
  renderWalletList();
  renderWalletSelector();
}

function getProfileWalletMap() {
  // Map of profile -> [wallet_address], stored locally
  try {
    return JSON.parse(localStorage.getItem("kc_profile_wallets") || "{}");
  } catch { return {}; }
}

function saveProfileWalletMap(map) {
  localStorage.setItem("kc_profile_wallets", JSON.stringify(map));
}

function assignWalletToProfile(walletAddress, profileId) {
  const map = getProfileWalletMap();
  if (!map[profileId]) map[profileId] = [];
  if (!map[profileId].includes(walletAddress)) {
    map[profileId].push(walletAddress);
  }
  saveProfileWalletMap(map);
}

function unassignWalletFromProfile(walletAddress, profileId) {
  const map = getProfileWalletMap();
  if (map[profileId]) {
    map[profileId] = map[profileId].filter(a => a !== walletAddress);
  }
  saveProfileWalletMap(map);
}

function getWalletsForProfile(profileId) {
  const map = getProfileWalletMap();
  const assigned = map[profileId] || [];
  // Return wallets that exist in backend AND are assigned to this profile
  // Plus unassigned wallets shown at bottom
  return {
    assigned: state.wallets.filter(w => assigned.includes(w.wallet_address)),
    unassigned: state.wallets.filter(w => {
      // Not assigned to ANY profile
      const allAssigned = Object.values(map).flat();
      return !allAssigned.includes(w.wallet_address);
    }),
  };
}

function renderWalletList() {
  const container = elements.walletListContainer;
  container.innerHTML = "";
  const { assigned, unassigned } = getWalletsForProfile(state.activeProfile);
  const all = [...assigned, ...unassigned];

  if (all.length === 0) {
    container.innerHTML = '<div class="wallet-card wallet-card--empty">No wallets yet. Create one below.</div>';
    return;
  }

  for (const w of all) {
    const isAssigned = assigned.includes(w);
    const isActive = w.wallet_address === state.activeWallet;
    const card = document.createElement("div");
    card.className = "wallet-card" + (isActive ? " wallet-card--active" : "");
    const shortAddr = w.wallet_address.slice(0, 8) + "\u2026" + w.wallet_address.slice(-6);
    const labelHtml = w.label
      ? `<div class="wc-label" title="Click to rename">${w.label}</div>`
      : `<div class="wc-label wc-label--empty" title="Click to name">unnamed</div>`;
    const userLabel = w.bound_user_id ? `<span class="wc-user">${w.bound_user_id}</span>` : "";
    const profileLabel = isAssigned
      ? `<span class="wc-profile wc-profile--mine">\u2713 ${getProfileName(state.activeProfile)}</span>`
      : `<span class="wc-profile wc-profile--none">unassigned</span>`;
    const shortPubKey = w.public_key ? w.public_key.slice(0, 8) + "\u2026" + w.public_key.slice(-6) : "";
    card.innerHTML = `
      ${labelHtml}
      <div class="wc-address" title="${w.wallet_address}">${shortAddr}</div>
      <div class="wc-meta">${w.chain} ${userLabel} ${profileLabel}</div>
      ${shortPubKey ? `<div class="wc-pubkey" title="${w.public_key}">pk: ${shortPubKey}</div>` : ""}
      <div class="wc-actions">
        <button class="wc-select-btn secondary" data-addr="${w.wallet_address}">Use</button>
        <button class="wc-rename-btn icon-btn" data-addr="${w.wallet_address}" title="Rename">✎</button>
        ${isAssigned
          ? `<button class="wc-unassign-btn icon-btn" data-addr="${w.wallet_address}" title="Remove from profile">&minus;</button>`
          : `<button class="wc-assign-btn icon-btn" data-addr="${w.wallet_address}" title="Assign to profile">&plus;</button>`
        }
      </div>
    `;
    container.appendChild(card);
  }

  // Wire card buttons
  container.querySelectorAll(".wc-select-btn").forEach(btn => {
    btn.addEventListener("click", () => selectActiveWallet(btn.dataset.addr));
  });
  container.querySelectorAll(".wc-rename-btn").forEach(btn => {
    btn.addEventListener("click", () => onRenameWallet(btn.dataset.addr));
  });
  container.querySelectorAll(".wc-label").forEach(lbl => {
    lbl.style.cursor = "pointer";
    lbl.addEventListener("click", () => {
      const card = lbl.closest(".wallet-card");
      const addr = card.querySelector(".wc-rename-btn")?.dataset.addr;
      if (addr) onRenameWallet(addr);
    });
  });
  container.querySelectorAll(".wc-assign-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      assignWalletToProfile(btn.dataset.addr, state.activeProfile);
      renderWalletList();
      renderWalletSelector();
    });
  });
  container.querySelectorAll(".wc-unassign-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      unassignWalletFromProfile(btn.dataset.addr, state.activeProfile);
      renderWalletList();
      renderWalletSelector();
    });
  });
}

function getProfileName(profileId) {
  const p = state.profiles.find(p => p.id === profileId);
  return p ? p.name : profileId;
}

function renderWalletSelector() {
  const sel = elements.activeWalletSelect;
  sel.innerHTML = "";
  const { assigned, unassigned } = getWalletsForProfile(state.activeProfile);
  const all = [...assigned, ...unassigned];

  if (all.length === 0) {
    const opt = document.createElement("option");
    opt.value = "";
    opt.textContent = "\u2014 no wallets \u2014";
    sel.appendChild(opt);
    return;
  }

  // Assigned wallets first
  if (assigned.length > 0) {
    const grp = document.createElement("optgroup");
    grp.label = "My Wallets";
    for (const w of assigned) {
      const opt = document.createElement("option");
      opt.value = w.wallet_address;
      const shortAddr = w.wallet_address.slice(0, 8) + "\u2026" + w.wallet_address.slice(-6);
      opt.textContent = w.label
        ? w.label + " \u2014 " + shortAddr
        : shortAddr + (w.bound_user_id ? " (" + w.bound_user_id + ")" : "");
      if (w.wallet_address === state.activeWallet) opt.selected = true;
      grp.appendChild(opt);
    }
    sel.appendChild(grp);
  }
  if (unassigned.length > 0) {
    const grp = document.createElement("optgroup");
    grp.label = "Unassigned";
    for (const w of unassigned) {
      const opt = document.createElement("option");
      opt.value = w.wallet_address;
      const shortAddr = w.wallet_address.slice(0, 8) + "\u2026" + w.wallet_address.slice(-6);
      opt.textContent = w.label
        ? w.label + " \u2014 " + shortAddr
        : shortAddr;
      if (w.wallet_address === state.activeWallet) opt.selected = true;
      grp.appendChild(opt);
    }
    sel.appendChild(grp);
  }
}

function selectActiveWallet(addr) {
  state.activeWallet = addr;
  localStorage.setItem("kc_active_wallet", addr);
  // Update selector
  elements.activeWalletSelect.value = addr;
  // Auto-populate all address fields
  elements.connectWalletAddress.value = addr;
  elements.balanceWalletAddress.value = addr;
  elements.signWalletAddress.value = addr;
  elements.submitFrom.value = addr;
  elements.fdWalletAddress.value = addr;
  elements.pcWalletAddress.value = addr;
  // Update half-fold info bar
  updateHalfFoldInfo();
  // Re-render wallet list to highlight active
  renderWalletList();
}

function updateHalfFoldInfo() {
  const w = state.wallets.find(w => w.wallet_address === state.activeWallet);
  if (w) {
    const name = w.label || w.wallet_address.slice(0, 8) + "\u2026" + w.wallet_address.slice(-6);
    elements.halfFoldWalletName.textContent = name;
    elements.halfFoldChain.textContent = w.chain;
  } else {
    elements.halfFoldWalletName.textContent = "\u2014";
    elements.halfFoldChain.textContent = "flowcortex-l1";
  }
}

function onWalletSelectorChange() {
  const addr = elements.activeWalletSelect.value;
  if (addr) selectActiveWallet(addr);
}

async function onChainConfig() {
  try {
    const result = await request("/chain/config", { method: "GET", headers: {} });
    setResult(elements.chainConfigResult, result);
  } catch (error) {
    setResult(elements.chainConfigResult, error.message, true);
  }
}

async function onWalletStatus() {
  try {
    const addr = elements.fdWalletAddress.value.trim();
    if (!addr) throw new Error("wallet address required");
    const result = await request("/fortressdigital/wallet-status", {
      method: "POST",
      body: JSON.stringify({
        wallet_address: addr,
        chain: "flowcortex-l1",
      }),
    });
    setResult(elements.walletStatusResult, result);
  } catch (error) {
    setResult(elements.walletStatusResult, error.message, true);
  }
}

async function onCommitment() {
  try {
    const addr = elements.pcWalletAddress.value.trim();
    const challenge = elements.pcChallenge.value.trim();
    if (!addr) throw new Error("wallet address required");
    if (!challenge) throw new Error("challenge required");
    const body = {
      wallet_address: addr,
      challenge: challenge,
      verification_result: true,
      chain: "flowcortex-l1",
    };
    const txHash = elements.pcTxHash.value.trim();
    if (txHash) body.tx_hash = txHash;
    const result = await request("/proofcortex/commitment", {
      method: "POST",
      body: JSON.stringify(body),
    });
    setResult(elements.commitmentResult, result);
  } catch (error) {
    setResult(elements.commitmentResult, error.message, true);
  }
}

async function onOpsHealth() {
  try {
    const result = await request("/health", { method: "GET", headers: {} });
    setResult(elements.opsResult, result);
  } catch (error) {
    setResult(elements.opsResult, error.message, true);
  }
}

async function onOpsReadyz() {
  try {
    const result = await request("/readyz", { method: "GET", headers: {} });
    setResult(elements.opsResult, result);
  } catch (error) {
    setResult(elements.opsResult, error.message, true);
  }
}

async function onOpsStartupz() {
  try {
    const result = await request("/startupz", { method: "GET", headers: {} });
    setResult(elements.opsResult, result);
  } catch (error) {
    setResult(elements.opsResult, error.message, true);
  }
}

function bindEvents() {
  for (const tab of elements.tabs) {
    tab.addEventListener("click", () => setActiveTab(tab.dataset.tab));
  }

  elements.createWalletBtn.addEventListener("click", onCreateWallet);
  elements.refreshWalletsBtn.addEventListener("click", loadWalletList);
  elements.restoreWalletBtn.addEventListener("click", onRestoreWallet);
  elements.profileSelect.addEventListener("change", onProfileChange);
  elements.addProfileBtn.addEventListener("click", onAddProfile);
  elements.activeWalletSelect.addEventListener("change", onWalletSelectorChange);
  elements.challengeBtn.addEventListener("click", onChallenge);
  elements.verifyBtn.addEventListener("click", onVerify);
  elements.bindWalletBtn.addEventListener("click", onBindWallet);
  elements.balanceBtn.addEventListener("click", onFetchBalance);
  elements.signBtn.addEventListener("click", onSignPayload);
  elements.nonceBtn.addEventListener("click", onFetchNonce);
  elements.submitTxBtn.addEventListener("click", onSubmitTx);
  elements.txStatusBtn.addEventListener("click", onFetchTxStatus);

  // Platform integration
  elements.chainConfigBtn.addEventListener("click", onChainConfig);
  elements.walletStatusBtn.addEventListener("click", onWalletStatus);
  elements.commitmentBtn.addEventListener("click", onCommitment);
  elements.healthBtn.addEventListener("click", onOpsHealth);
  elements.readyzBtn.addEventListener("click", onOpsReadyz);
  elements.startupzBtn.addEventListener("click", onOpsStartupz);

  elements.balanceAsset.addEventListener("change", updateBalanceIcons);
  elements.balanceChain.addEventListener("input", updateBalanceIcons);

  elements.skinSelect.addEventListener("change", () => {
    const skin = elements.skinSelect.value;
    applySkin(skin);
    localStorage.setItem("kc_wallet_skin", skin);
  });

  elements.skinCycleBtn.addEventListener("click", cycleSkin);
}

async function main() {
  const savedSkin = localStorage.getItem("kc_wallet_skin");
  if (savedSkin && elements.skinSelect.querySelector(`option[value="${savedSkin}"]`)) {
    elements.skinSelect.value = savedSkin;
  }
  applySkin(elements.skinSelect.value);

  // Load profiles and wallets
  loadProfiles();
  await loadWalletList();

  // Restore last active wallet
  const savedWallet = localStorage.getItem("kc_active_wallet");
  if (savedWallet && state.wallets.some(w => w.wallet_address === savedWallet)) {
    selectActiveWallet(savedWallet);
  } else if (state.wallets.length > 0) {
    selectActiveWallet(state.wallets[0].wallet_address);
  }

  bindEvents();
  await loadManifest();
}

main();
