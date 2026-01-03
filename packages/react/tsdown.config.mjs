import { defineConfig } from "tsdown";

export default defineConfig({
  entry: "src/index.ts",
  minify: true,
  platform: "neutral",
  inputOptions: {
    transform: {
      jsx: "react",
    },
  },
  outDir: "dist",
  dts: true,
});
