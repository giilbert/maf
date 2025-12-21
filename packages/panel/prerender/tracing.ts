// a simple logger inspired by Rust's tracing crate
// TODO: make this better and publish it as its own package

import chalk from "chalk";

const log = (...other: unknown[]) => {
  console.log(chalk.blue(`[prerender]`), ...other);
};

const span = (name: string) => {
  return (...other: unknown[]) => {
    log(chalk.dim(`<${name}>`), ...other);
  };
};

export const tracing = {
  log,
  span,
};
