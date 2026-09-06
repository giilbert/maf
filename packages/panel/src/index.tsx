import {
  createRootRouteWithContext,
  createRoute,
  createRouter,
  lazyRouteComponent,
  Link,
  Outlet,
  redirect,
} from "@tanstack/react-router";
import { TanStackRouterDevtools } from "@tanstack/react-router-devtools";
import "./globals.css";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fetchSessionInfo, GET_SESSION_QUERY_KEY } from "./lib/auth";

export interface RouterContext {
  queryClient: QueryClient;
}

const queryClient = new QueryClient({});

const rootRoute = createRootRouteWithContext<RouterContext>()({
  beforeLoad: async ({ context }) => {
    // Load the session before EVERYTHING else so that we can use it in the
    // loader and avoid odd flickers.
    const session = await context.queryClient.fetchQuery({
      queryKey: GET_SESSION_QUERY_KEY,
      queryFn: fetchSessionInfo,
    });

    return { session };
  },
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
  component: lazyRouteComponent(() => import("./routes/home"), "HomePage"),
});

const loginRoute = createRoute({
  path: "/login",
  getParentRoute: () => rootRoute,
  component: lazyRouteComponent(() => import("./routes/login"), "LoginPage"),
  loader: async ({ context }) => {
    if (context.session) throw redirect({ to: "/" });
  },
});

export const routeTree = rootRoute.addChildren([homeRoute, loginRoute]);

export const createAppRouter = () =>
  createRouter({
    routeTree,
    basepath: "/~",
    context: { queryClient },
    defaultNotFoundComponent: () => <h1>Not Found</h1>,
    Wrap: ({ children }) => {
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
