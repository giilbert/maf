import ReactDomServer from "react-dom/server";
import { RouterServer } from "@tanstack/react-router/ssr/server";
import { AnyRoute, createMemoryHistory } from "@tanstack/react-router";
import path from "node:path";
import chalk from "chalk";

import { tracing } from "./tracing";
import { createAppRouter } from "../src";
import { serve } from "./serve";

if (Bun.argv[2] === "serve") await serve();

// check that dist/index.html exists
const DIST_PATH = path.join(__dirname, "../dist");
const PRERENDER_OUTPUT_PATH = path.join(DIST_PATH, "prerender");
const INDEX_HTML_PATH = path.join(DIST_PATH, "index.html");

const htmlFile = Bun.file(INDEX_HTML_PATH);
const htmlContentPromise = htmlFile.text();

interface RouteDescription {
  route: AnyRoute;
}

export interface OutputFile {
  route: string;
  outputFilePath: string;
}

export interface Manifest {
  basepath: string;
  files: OutputFile[];
}

interface Ctx {
  manifest: Manifest;
}

const preRenderRoute = async (ctx: Ctx, route: AnyRoute) => {
  const router = createAppRouter();
  // need to normalize the full path to use :param instead of $param
  const routePath = `${router.basepath}${route.fullPath}`.replace("$", ":");

  const log = tracing.span(`route ${routePath}`);
  log(chalk.dim(`starting prerender at ${new Date().toLocaleString()}`));
  const start = Date.now();

  // we want to render only the shell if (OR):
  // - the route has a loader
  // - the route has params
  // otherwise, we can render the full content at prerender time
  const hasLoader = !!route.options.loader;
  const hasParams = route.fullPath.includes("$");
  const isShell = hasLoader || hasParams;

  router.update({
    history: createMemoryHistory({ initialEntries: [routePath] }),
    isShell,
  });

  await router.load();

  const markup = ReactDomServer.renderToString(
    <RouterServer router={router} />
  );

  const htmlContent = await htmlContentPromise;
  const finalHtml = htmlContent.replace("<!-- shell -->", `${markup}`);

  const outputFilePath = path.join(
    PRERENDER_OUTPUT_PATH,
    route.fullPath === "/" ? "index.html" : `${route.fullPath}.html`
  );
  await Bun.write(outputFilePath, finalHtml);

  ctx.manifest.files.push({
    // TODO: better params logic
    route: route.fullPath.replace("$", ":"),
    outputFilePath: outputFilePath.replace(PRERENDER_OUTPUT_PATH, ""),
  });

  const end = Date.now();
  log(`finished prerender of ${routePath} in ${end - start}ms`);
};

if (await htmlFile.exists()) {
  tracing.log(`found index.html at ${INDEX_HTML_PATH}`);

  // set prerendering up
  await Bun.$`rm -rf ${PRERENDER_OUTPUT_PATH}`;
  const router = createAppRouter();
  const allRoutes: RouteDescription[] = [];

  const walk = (route: AnyRoute) => {
    if (!route.isRoot) allRoutes.push({ route });
    for (const child of route.children || []) walk(child);
  };
  walk(router.routeTree);

  tracing.log(`discovered ${allRoutes.length} routes`);
  const ctx: Ctx = { manifest: { basepath: router.basepath, files: [] } };
  for (const { route } of allRoutes) await preRenderRoute(ctx, route);

  tracing.log();
  tracing.log(`generated ${ctx.manifest.files.length} files:`);
  const firstColumnMaxLength = Math.max(
    ...ctx.manifest.files.map((f) => f.route.length)
  );
  for (const file of ctx.manifest.files) {
    tracing.log(
      ` - route: ${chalk.cyan(file.route.padEnd(firstColumnMaxLength, " "))} -> ${chalk.green(file.outputFilePath)}`
    );
  }

  Bun.file(path.join(PRERENDER_OUTPUT_PATH, "manifest.json")).write(
    JSON.stringify(ctx.manifest, null, 2)
  );

  tracing.log(`manifest written to ${chalk.green("prerender/manifest.json")}`);
  tracing.log();

  // optionally serve the prerendered files
  if (Bun.argv[2] === "--serve") await serve();
} else {
  console.error("could not find index.html at", INDEX_HTML_PATH);
  console.error("hint: did you build @usemaf/panel with `pnpm build`?");
  process.exit(1);
}
