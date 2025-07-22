import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { MafProvider } from "@usemaf/client/src/maf-provider";
import "./index.css";
import App from "./App.tsx";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <MafProvider url="http://localhost:1147" app="santiago/game-clock">
      <App />
    </MafProvider>
  </StrictMode>
);
