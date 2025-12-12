import { defineConfig } from "tsdown";

export default defineConfig({
  entry: "src/index.ts",
  minify: true,
  platform: "neutral",
  outDir: "dist",
  dts: true,
});
