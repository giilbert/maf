import { componentize } from "@bytecodealliance/componentize-js";
import { Args, Command, Flags } from "@oclif/core";
import { build } from "esbuild";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import * as url from "node:url";

export default class Compile extends Command {
  static args = {
    entry: Args.string({
      description: "Entry file",
      required: true,
    }),
  };
  static description = "Compile a WebAssembly module for MAF";
  static examples = [];
  static flags = {};

  async run(): Promise<void> {
    const { args, flags } = await this.parse(Compile);

    const fileName = url.fileURLToPath(import.meta.url);
    const dirName = path.dirname(fileName);
    const witPath = path.join(dirName, "../../../../../wit");

    console.log("running esbuild...");

    const buildResult = await build({
      entryPoints: [args.entry],
      bundle: true,
      platform: "neutral",
      format: "esm",
      preserveSymlinks: true,
      outfile: "build/out.js",
      external: ["wasi:io/poll@0.2.4", "maf:bindings/bindings"],
    });

    console.log("esbuild done.");

    const result = await componentize({
      sourcePath: "build/out.js",
      // TODO: handle this better
      witPath,
      worldName: "imports",
      debugBuild: true,
      disableFeatures: ["http"],
    });

    const importMap: Record<string, string[]> = {};
    for (const i of result.imports) {
      const wasmImport = i as unknown as [string, string];

      const wasmImportModule = wasmImport[0];
      const wasmImportName = wasmImport[1];

      if (!importMap[wasmImportModule]) importMap[wasmImportModule] = [];
      importMap[wasmImportModule].push(wasmImportName);
    }

    for (const [module, imports] of Object.entries(importMap)) {
      console.log(`${module}:`);

      for (const importName of imports) {
        console.log(` - ${importName}`);
      }
    }

    await fs.writeFile("build/out.wasm", result.component, "binary");
  }
}
