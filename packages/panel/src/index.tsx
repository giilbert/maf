import {
  createRootRouteWithContext,
  createRoute,
  createRouter,
  Link,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
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
        <Link to="/login">Go to Login</Link>
      </>
    );
  },
});

const loginRoute = createRoute({
  path: "/login",
  getParentRoute: () => rootRoute,
  component: () => {
    return <p>Login Page</p>;
  },
});

const routeTree = rootRoute.addChildren([homeRoute, loginRoute]);
const router = createRouter({ routeTree, basepath: "/~" });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>
);
