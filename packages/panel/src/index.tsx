import {
  createRootRouteWithContext,
  createRoute,
  createRouter,
  lazyRouteComponent,
  Link,
  Outlet,
} from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import "./globals.css";

const rootRoute = createRootRouteWithContext()({
  component: () => {
    return (
      <div className="p-8">
        <Outlet />
        <TanStackRouterDevtools initialIsOpen={false} />
      </div>
    );
  },
});

const homeRoute = createRoute({
  path: "/",
  getParentRoute: () => rootRoute,
  component: () => {
    return (
      <>
        <h1>Home Page</h1>
        <Link to="/loader">Go to Loader</Link>
      </>
    );
  },
});

const loaderRoute = createRoute({
  path: "/loader",
  getParentRoute: () => rootRoute,
  loader: async () => {
    await new Promise((r) => setTimeout(r, 1000));
    return { message: "hello from the loader!" };
  },
  pendingComponent: () => <p>/loader is pending...</p>,
  component: lazyRouteComponent(() => import("./routes/loader"), "LoaderPage"),
});

const dynamicRoute = createRoute({
  path: "/dynamic/$param",
  getParentRoute: () => rootRoute,
  pendingComponent: () => <p>/dynamic is pending...</p>,
  component: lazyRouteComponent(
    () => import("./routes/dynamic"),
    "DynamicPage"
  ),
});

export const routeTree = rootRoute.addChildren([
  homeRoute,
  loaderRoute,
  dynamicRoute,
]);

export const createAppRouter = () =>
  createRouter({
    routeTree,
    basepath: "/~",
    defaultNotFoundComponent: () => <h1>Not Found</h1>,
  });

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof createAppRouter>;
  }
}
