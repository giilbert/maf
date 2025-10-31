import { componentize } from "@bytecodealliance/componentize-js";
import { Args, Command } from "@oclif/core";
import { colorize } from "@oclif/core/ux";
import { build } from "esbuild";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as url from "node:url";

const formatMs = (ms: number): string => {
  if (ms < 1000) return `${ms}ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(2)}s`;
  const m = Math.floor(s / 60);
  const sRemainder = s % 60;
  return `${m}m ${sRemainder.toFixed(2)}s`;
};

const generateEntryWrapper = (absEntryPath: string): string => {
  return `import { app } from "${absEntryPath}";

export function run() {
  app.run();
}

export function dryRun() {
  console.log("dryRun() called");
}`;
};

export default class Compile extends Command {
  static args = {
    entry: Args.string({
      description: "Entry file",
      required: true,
    }),
  };
  static description = "Compile a WebAssembly module for Cobble";
  static examples = [];
  static flags = {};

  async run(): Promise<void> {
    const { args } = await this.parse(Compile);

    const fileName = url.fileURLToPath(import.meta.url);
    const dirName = path.dirname(fileName);
    const witPath = path.join(dirName, "../../../../../crates/cobble/wit");

    const esbuildStart = Date.now();
    const buildEntry = "build/entry.ts";
    await fs.writeFile(
      buildEntry,
      generateEntryWrapper(path.resolve(args.entry))
    );

    console.log(
      colorize("gray", `[bundle] Running \`esbuild\` on ${args.entry}...`)
    );

    await build({
      entryPoints: [buildEntry],
      bundle: true,
      platform: "neutral",
      format: "esm",
      preserveSymlinks: true,
      outfile: "build/out.js",
      external: ["wasi:io/poll@0.2.6", "cobble:bindings/bindings"],
    });

    console.log(
      `[bundle] \`esbuild\` done in ${formatMs(Date.now() - esbuildStart)}`
    );

    const componentizeStart = Date.now();
    console.log(
      colorize("gray", `[wasify] Running componentize on build output...`)
    );
    const result = await componentize({
      sourcePath: "build/out.js",
      // TODO: handle this better
      witPath,
      worldName: "imports",
      debugBuild: true,
      disableFeatures: ["http"],
    });
    console.log(
      `[wasify] componentize done in ${formatMs(
        Date.now() - componentizeStart
      )}`
    );
    console.log(
      colorize(
        "gray",
        `[wasify] Generated component with ${result.imports.length} imports:`
      )
    );

    const importMap: Record<string, string[]> = {};
    for (const i of result.imports) {
      const wasmImport = i as unknown as [string, string];

      const wasmImportModule = wasmImport[0];
      const wasmImportName = wasmImport[1];

      if (!importMap[wasmImportModule]) importMap[wasmImportModule] = [];
      importMap[wasmImportModule].push(wasmImportName);
    }

    for (const [module, imports] of Object.entries(importMap)) {
      console.log(colorize("gray", `${module}:`));

      for (const importName of imports) {
        console.log(colorize("gray", ` - ${importName}`));
      }
    }

    await fs.writeFile("build/out.wasm", result.component, "binary");
    console.log("[wasify] Wrote `build/out.wasm`!");
  }
}
