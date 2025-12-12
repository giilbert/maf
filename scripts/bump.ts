// Bumps the version of all packages in the monorepo to the specified version.

import * as path from "node:path";
import chalk from "chalk";

const PROJECT_ROOT = path.resolve(import.meta.dirname, "..");
const PACKAGES_DIR = path.join(PROJECT_ROOT, "packages");

const packages = [];

const pattern = new Bun.Glob(`*/package.json`);
for await (const path of pattern.scan({
  cwd: PACKAGES_DIR,
  absolute: true,
})) {
  const packageJson = JSON.parse(await Bun.file(path).text());
  packages.push({ name: packageJson.name, version: packageJson.version, path });
}

const version = process.argv[2];
if (!version) {
  console.error("invalid usage! `bun run scripts/bump.ts <version>`");
  process.exit(1);
}

let nameMaxWidth = packages.reduce(
  (acc, pkg) => Math.max(acc, pkg.name.length),
  0
);

console.group("the following packages will be modified:");
for (const pkg of packages) {
  console.log(
    chalk.cyan(pkg.name.padEnd(nameMaxWidth, " ")),
    ":",
    chalk.yellow(pkg.version),
    "-->",
    chalk.green(version)
  );
}
console.groupEnd();

console.log("");
const input = prompt(chalk.bold("confirm? (y/N)")) || "";
if (input.trim().toLowerCase() !== "y") {
  console.warn(chalk.yellow("aborting."));
  process.exit(0);
}

for (const pkg of packages) {
  const packageJson = JSON.parse(await Bun.file(pkg.path).text());
  packageJson.version = version;
  await Bun.write(pkg.path, JSON.stringify(packageJson, null, 2) + "\n");
}
