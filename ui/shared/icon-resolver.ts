export type IconEntry = {
  label: string;
  icon: string;
};

export type IconManifest = {
  version: string;
  updated_at: string;
  defaults: {
    network: string;
    coin: string;
  };
  networks: Record<string, IconEntry>;
  coins: Record<string, IconEntry>;
  mvp_constraints?: {
    active_chain?: string;
    active_assets?: string[];
  };
};

export type ResolvedIcon = {
  key: string;
  label: string;
  icon: string;
  isFallback: boolean;
};

const normalize = (value: string): string => value.trim();

const normalizeCoin = (value: string): string => {
  const trimmed = value.trim();
  if (trimmed.toLowerCase() === "flower") {
    return "FloweR";
  }
  return trimmed.toUpperCase();
};

export function resolveNetworkIcon(
  manifest: IconManifest,
  chain: string,
): ResolvedIcon {
  const key = normalize(chain);
  const entry = manifest.networks[key];

  if (entry) {
    return {
      key,
      label: entry.label,
      icon: entry.icon,
      isFallback: false,
    };
  }

  return {
    key,
    label: key,
    icon: manifest.defaults.network,
    isFallback: true,
  };
}

export function resolveCoinIcon(
  manifest: IconManifest,
  asset: string,
): ResolvedIcon {
  const key = normalizeCoin(asset);
  const entry = manifest.coins[key];

  if (entry) {
    return {
      key,
      label: entry.label,
      icon: entry.icon,
      isFallback: false,
    };
  }

  return {
    key,
    label: key,
    icon: manifest.defaults.coin,
    isFallback: true,
  };
}

export function isMvpChainAllowed(
  manifest: IconManifest,
  chain: string,
): boolean {
  const activeChain = manifest.mvp_constraints?.active_chain;
  if (!activeChain) {
    return true;
  }
  return normalize(chain) === activeChain;
}

export function isMvpAssetAllowed(
  manifest: IconManifest,
  asset: string,
): boolean {
  const activeAssets = manifest.mvp_constraints?.active_assets;
  if (!activeAssets || activeAssets.length === 0) {
    return true;
  }

  const normalizedAsset = normalizeCoin(asset);
  return activeAssets.some((item) => normalizeCoin(item) === normalizedAsset);
}

export function resolveWalletVisuals(
  manifest: IconManifest,
  chain: string,
  asset: string,
): {
  network: ResolvedIcon;
  coin: ResolvedIcon;
  allowedInMvp: boolean;
} {
  const network = resolveNetworkIcon(manifest, chain);
  const coin = resolveCoinIcon(manifest, asset);

  return {
    network,
    coin,
    allowedInMvp:
      isMvpChainAllowed(manifest, chain) && isMvpAssetAllowed(manifest, asset),
  };
}
