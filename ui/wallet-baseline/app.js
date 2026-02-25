const state = {
  manifest: null,
};

const byId = (id) => document.getElementById(id);

const elements = {
  walletWindow: document.querySelector(".wallet-window"),
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

  txHash: byId("txHash"),
  txStatusBtn: byId("txStatusBtn"),
  historyResult: byId("historyResult"),
};

function baseUrl() {
  return elements.baseUrl.value.trim().replace(/\/+$/, "");
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
  root.style.setProperty('--wallet-skin-size', '4.8px 4.8px');
  root.style.setProperty('--edge-color', theme.edge);
  root.style.setProperty('--edge-color-soft', theme.secondary);
  root.style.setProperty('--glass-bg', theme.glass);
  root.style.setProperty('--wallet-text', theme.text);
  root.style.setProperty('--wallet-accent', theme.accent);
}

async function applySkin(skin) {
  const root = elements.walletWindow;
  root.classList.remove("skin-dark", "skin-black");
  // Load theme tokens
  const allThemes = themes || await loadThemes();
  const theme = allThemes[skin] || allThemes.classic;
  setThemeVars(theme);
  if (skin === "dark") {
    root.classList.add("skin-dark");
  } else if (skin === "black") {
    root.classList.add("skin-black");
  }
}

function cycleSkin() {
  const skins = ["classic", "dark", "black"];
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
    const result = await request("/wallet/create", { method: "POST" });
    setResult(elements.createResult, result);

    const walletAddress = result.wallet_address || "";
    elements.connectWalletAddress.value = walletAddress;
    elements.balanceWalletAddress.value = walletAddress;
    elements.signWalletAddress.value = walletAddress;
  } catch (error) {
    setResult(elements.createResult, error.message, true);
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

function bindEvents() {
  for (const tab of elements.tabs) {
    tab.addEventListener("click", () => setActiveTab(tab.dataset.tab));
  }

  elements.createWalletBtn.addEventListener("click", onCreateWallet);
  elements.bindWalletBtn.addEventListener("click", onBindWallet);
  elements.balanceBtn.addEventListener("click", onFetchBalance);
  elements.signBtn.addEventListener("click", onSignPayload);
  elements.txStatusBtn.addEventListener("click", onFetchTxStatus);

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

  bindEvents();
  await loadManifest();
}

main();
