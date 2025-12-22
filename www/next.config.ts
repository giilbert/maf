import { type NextConfig } from "next";
import fs from "node:fs/promises";
import path from "node:path";
import type { Manifest } from "@usemaf/panel/prerender";

const createRewrites = async () => {
  const panelDir = path.join(__dirname, "public/_panel");
  const doesPanelExist = await fs.stat(panelDir);
  if (!doesPanelExist) return [];

  console.log("-> [panel] configuring rewrites for /_panel static assets.");
  const manifest: Manifest = JSON.parse(
    await fs.readFile(path.join(panelDir, "prerender/manifest.json"), "utf-8")
  );

  console.log(`-> [panel] found ${manifest.files.length} prerendered files.`);

  const result = [
    ...manifest.files.map((file) => ({
      source: `${manifest.basepath}${file.route.replace(/\/$/, "")}`,
      destination: `/_panel/prerender${file.outputFilePath}`,
    })),
    {
      source: "/~/assets/:path*",
      destination: "/_panel/assets/:path*",
    },
  ];
  for (const rewrite of result) {
    console.log(
      `-> [panel] rewrite: ${rewrite.source} -> ${rewrite.destination}`
    );
  }
  return result;
};

export default {
  pageExtensions: ["js", "jsx", "ts", "tsx", "md", "mdx"],
  // Route /~/* to a SPA at /_panel/*
  rewrites: createRewrites,
} satisfies NextConfig;
