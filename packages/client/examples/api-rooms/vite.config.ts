import { defineConfig } from "vite";
import apiRoutes from "vite-plugin-api-routes";

export default defineConfig({
  plugins: [
    apiRoutes({
      mode: "isolated",
    }),
  ],
});
