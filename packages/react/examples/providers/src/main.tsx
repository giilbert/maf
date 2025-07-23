import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { MafProvider } from "@usemaf/react";
import App from "./app";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <MafProvider url="http://localhost:1147" app="santiago/game-clock">
      <App />
    </MafProvider>
  </StrictMode>
);
