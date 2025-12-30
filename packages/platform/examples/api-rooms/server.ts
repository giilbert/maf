import { MafServiceClient } from "@usemaf/platform";
import { Hono } from "hono";
import { cors } from "hono/cors";

const server = new MafServiceClient({
  // You would likely have a import.meta.env switch here to choose between
  // dev/prod servers.
  // server: "dev",
  // Uncomment to use local platform server
  server: {
    type: "platform",
    app: "gilbert/example-platform",
    url: "http://localhost:1147",
  },

  // Fake credentials for testing purposes
  // In a real application, you would use actual credentials
  clientId: "test-client-id",
  clientSecret: "secret",
});

const app = new Hono();

app.use("*", cors({ origin: "*" }));

app
  .post("/api/rooms", async (c) => {
    try {
      // Call to MAF Platform to create a new room
      const room = await server.rooms.create();
      console.log("Created room", room);
      return c.json({ type: "success", data: room });
    } catch (e) {
      console.error("Failed to create room:", e);
      return c.json({ type: "error" }, 500);
    }
  })
  .get("/api/rooms", async (c) => {
    try {
      // Call to MAF Platform to list rooms
      const rooms = await server.rooms.list();
      console.log("Listed rooms", rooms);
      return c.json(rooms.map((room) => ({ id: room.id })));
    } catch (e) {
      console.error("Failed to list rooms:", e);
      return c.json({ type: "error" }, 500);
    }
  });

export default {
  fetch: app.fetch,
  port: 8080,
};
