import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { MafProvider } from "@usemaf/react";
import { App } from "./app";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <MafProvider server="dev">
      <App />
    </MafProvider>
  </StrictMode>
);
