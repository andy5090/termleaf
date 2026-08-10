import { cp, readFile, readdir, writeFile } from "node:fs/promises";
import { join, relative, sep } from "node:path";

const configPath = ".cloudflare/output/v0/workers/default/config.json";
const bundlePath = ".cloudflare/output/v0/workers/default/bundle";
const ssrSourcePath = "dist/server/ssr";
const ssrBundlePath = join(bundlePath, "ssr");
const config = JSON.parse(await readFile(configPath, "utf8"));

await cp(ssrSourcePath, ssrBundlePath, { recursive: true });

const workerEntryPath = join(bundlePath, "index.js");
const workerEntry = await readFile(workerEntryPath, "utf8");
await writeFile(
  workerEntryPath,
  workerEntry.replaceAll(
    "../../../../../../dist/server/ssr/index.js",
    "./ssr/index.js",
  ),
);

const ssrEntryPath = join(ssrBundlePath, "index.js");
const ssrEntry = await readFile(ssrEntryPath, "utf8");
await writeFile(
  ssrEntryPath,
  ssrEntry.replace(
    "../../../.cloudflare/output/v0/workers/default/bundle/index.js",
    "../index.js",
  ),
);

async function addJavaScriptModules(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);

    if (entry.isDirectory()) {
      await addJavaScriptModules(path);
    } else if (entry.name.endsWith(".js")) {
      const moduleName = relative(bundlePath, path).split(sep).join("/");
      config.manifest.modules[moduleName] = { type: "esm" };
    }
  }
}

await addJavaScriptModules(bundlePath);

await writeFile(configPath, `${JSON.stringify(config)}\n`);
