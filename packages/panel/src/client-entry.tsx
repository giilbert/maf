import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createAppRouter } from ".";

const router = createAppRouter();

Promise.all([router.load(), new Promise((r) => setTimeout(r, 200))]).then(
  () => {
    const root = createRoot(document.getElementById("root")!);
    root.render(
      <StrictMode>
        <RouterProvider router={router} />
      </StrictMode>
    );
  }
);
