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
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";

const rootRoute = createRootRouteWithContext()({
  component: () => {
    return (
      <div>
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
        <Link to="/login">Login</Link>
      </>
    );
  },
});

const loginRoute = createRoute({
  path: "/login",
  getParentRoute: () => rootRoute,
  component: lazyRouteComponent(() => import("./routes/login"), "LoginPage"),
});

export const routeTree = rootRoute.addChildren([homeRoute, loginRoute]);

export const createAppRouter = () =>
  createRouter({
    routeTree,
    basepath: "/~",
    defaultNotFoundComponent: () => <h1>Not Found</h1>,
    Wrap: ({ children }) => {
      const [queryClient] = useState(() => new QueryClient({}));
      return (
        <QueryClientProvider client={queryClient}>
          {children}
        </QueryClientProvider>
      );
    },
  });

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof createAppRouter>;
  }
}
