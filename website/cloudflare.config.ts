import { defineWorker } from "@cloudflare/vite-plugin/experimental-config";

export default defineWorker({
  name: "termleaf-writing",
  compatibilityDate: "2026-05-15",
  compatibilityFlags: ["nodejs_compat"],
  entrypoint: "./worker/index.ts",
  domains: ["termleaf.com", "www.termleaf.com"],
  workersDev: true,
  observability: {
    enabled: true,
  },
});
