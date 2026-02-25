import manifest from "../config/icon-manifest.json";
import {
  resolveNetworkIcon,
  resolveCoinIcon,
  resolveWalletVisuals,
} from "./icon-resolver";

const network = resolveNetworkIcon(manifest, "flowcortex-l1");
const coin = resolveCoinIcon(manifest, "FloweR");

const walletVisuals = resolveWalletVisuals(manifest, "flowcortex-l1", "PROOF");

console.log(network.icon);
console.log(coin.icon);
console.log(walletVisuals.allowedInMvp);
