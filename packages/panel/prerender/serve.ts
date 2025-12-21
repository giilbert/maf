import path from "node:path";
import { pathToRegexp } from "path-to-regexp";

import { tracing } from "./tracing";
import { Manifest, OutputFile } from ".";

const PRERENDER_OUTPUT_PATH = path.join(__dirname, "../dist/prerender");

const loadManifest = async () => {
  return (await Bun.file(
    path.join(PRERENDER_OUTPUT_PATH, "manifest.json")
  ).json()) as Manifest;
};

export const serve = async () => {
  const log = tracing.span("serve");
  const manifest = await loadManifest();

  log(`loaded ${manifest.files.length} routes:`);
  for (const entry of manifest.files) {
    log(` - ${entry.route}`);
  }

  // convert manifest entries to route regexps
  const routes: {
    regexp: RegExp;
    entry: OutputFile;
  }[] = [];
  for (const entry of manifest.files) {
    routes.push({
      regexp: pathToRegexp(entry.route).regexp,
      entry,
    });
  }

  const port = process.env.PORT ? Number(process.env.PORT) : 3000;
  const server = Bun.serve({
    port,
    fetch: async (request) => {
      const url = new URL(request.url);
      // need to clean the pathname by removing the basepath
      const pathname = url.pathname.replace(manifest.basepath, "") || "/";

      // forward asset requests
      if (pathname.startsWith("/assets")) {
        return new Response(
          Bun.file(path.join(PRERENDER_OUTPUT_PATH, "../", pathname))
        );
      }

      // this is very inefficient, but fine for development server stuff
      for (const route of routes) {
        if (route.regexp.test(pathname)) {
          return new Response(
            Bun.file(
              path.join(PRERENDER_OUTPUT_PATH, route.entry.outputFilePath)
            )
          );
        }
      }

      // in dev server, return a simple 404 in cases where **a route cannot be
      // matched**. in production, this would be handled by the router itself
      // and/or the application.
      log(` - no route matched for ${pathname} -> 404`);
      return new Response("404: Not Found", { status: 404 });
    },
  });

  log(`serving prerendered files at ${server.url}`);

  // hack to wait for bun server to end
  await new Promise(() => {});
};
