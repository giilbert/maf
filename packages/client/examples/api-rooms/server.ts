import { CobbleServiceClient } from "@usecobble/platform";
import { Hono } from "hono";
import { cors } from "hono/cors";

const server = new CobbleServiceClient({
  server: "dev",
  // Fake credentials for testing purposes
  // In a real application, you would use actual credentials
  clientId: "test-client-id",
  clientSecret: "secret",
});

const app = new Hono();

app.use("*", cors({ origin: "*" }));
app.post("/api/rooms", async (c) => {
  try {
    const data = await server.rooms.create();
    return c.json({ type: "success", data });
  } catch (e) {
    console.error("Failed to create room:", e);
    return c.json({ type: "error" }, 500);
  }
});

export default {
  fetch: app.fetch,
  port: 8080,
};
