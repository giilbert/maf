import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwind from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwind()],
  base: "/~",
  build: process.env.NODE_ENV === "development" ? { minify: false } : undefined,
});
