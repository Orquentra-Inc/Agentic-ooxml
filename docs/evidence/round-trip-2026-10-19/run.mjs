import { build } from "<MONOREPO>/apps/editors/pptx/extension/webview/node_modules/esbuild/lib/main.js";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
const W = "<MONOREPO>/apps/editors/pptx/extension/webview";
const out = mkdtempSync(join(tmpdir(), "aoexp-bundle-"));
await build({ entryPoints: [process.argv[2]], bundle: true, platform: "node", format: "esm", outdir: out, outExtension: { ".js": ".mjs" }, logLevel: "error",
  nodePaths: [W + "/node_modules"], alias: { "@eigenpal/docx-core/components": "<MONOREPO>/packages/docx-editor/core/src/components/index.ts" },
  banner: { js: "import { createRequire } from 'module'; const require = createRequire(import.meta.url);" } });
const f = readdirSync(out).filter((x) => x.endsWith(".mjs")).map((x) => join(out, x));
const r = spawnSync(process.execPath, ["--test", ...f], { stdio: "inherit", cwd: W });
process.exit(r.status ?? 1);
