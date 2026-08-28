import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createAppRouter } from ".";

const router = createAppRouter();

const setup = () => {
  const root = createRoot(document.getElementById("root")!);
  root.render(
    <StrictMode>
      <RouterProvider router={router} />
    </StrictMode>,
  );
};

Promise.all([
  router.load(),
  new Promise<void>((r) => {
    const shouldDelay = !import.meta.env.DEV;

    // delay if prerendered to avoid flickering the loading UI for too short
    if (shouldDelay) setTimeout(r, 200);
    else r();
  }),
]).then(setup);
