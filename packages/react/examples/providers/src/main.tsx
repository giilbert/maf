import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { MafProvider, useStore } from "@usemaf/react";
import { App } from "./app";
import type { MafApp } from "./types";

declare module "@usemaf/client" {
  interface MafTypes {
    generated: MafApp;
  }
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <MafProvider server="dev">
      <App />
    </MafProvider>
  </StrictMode>
);

useStore("count");
