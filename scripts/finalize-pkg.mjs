// Augment the wasm-pack-generated pkg/ with the WASI tool runner so the npm
// package ships both the browser library and the wbtools_oss tools.
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
const PKG = "crates/whitebox-wasm/pkg";
copyFileSync("target/wasm32-wasip1/release/whitebox.wasm", `${PKG}/whitebox-cli.wasm`);
copyFileSync("crates/whitebox-wasm/tools.mjs", `${PKG}/tools.mjs`);
copyFileSync("crates/whitebox-wasm/tools.d.ts", `${PKG}/tools.d.ts`);
const p = JSON.parse(readFileSync(`${PKG}/package.json`, "utf8"));
p.dependencies = { ...(p.dependencies || {}), "@bjorn3/browser_wasi_shim": "^0.4.2" };
const extra = ["whitebox-cli.wasm", "tools.mjs", "tools.d.ts"];
p.files = [...new Set([...(p.files || []), ...extra])];
p.exports = {
  ".": { types: "./whitebox_wasm.d.ts", default: "./whitebox_wasm.js" },
  "./tools": { types: "./tools.d.ts", default: "./tools.mjs" },
};
writeFileSync(`${PKG}/package.json`, JSON.stringify(p, null, 2) + "\n");
console.log("finalized pkg: + whitebox-cli.wasm, tools.mjs, browser_wasi_shim dep, ./tools export");
