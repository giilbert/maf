import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { CobbleProvider, useStore } from "@usecobble/react";
import { App } from "./app";
import type { CobbleApp } from "./types";
import "./index.css";

declare module "@usecobble/client" {
  interface CobbleTypes {
    generated: CobbleApp;
  }
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <CobbleProvider server="dev">
      <App />
    </CobbleProvider>
  </StrictMode>
);

useStore("count");
