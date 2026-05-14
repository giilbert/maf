import { MafServiceClient, Room } from "@usemaf/platform";
import { Hono } from "hono";
import { cors } from "hono/cors";

const server = new MafServiceClient({
  // You would likely have a import.meta.env switch here to choose between
  // dev/prod servers.
  server: "dev",
  // Uncomment to use local platform server
  // server: {
  //   type: "platform",
  //   app: "gilbert/example-platform",
  //   url: "http://localhost:1147",
  // },

  // Fake credentials for testing purposes
  // In a real application, you would use actual credentials
  clientId: "test-client-id",
  clientSecret: "secret",
});

const app = new Hono();

const printRoomInfo = (room: Room) => {
  console.log("- Room ID:", room.id);
  console.log("  Meta:");
  for (const [key, entry] of Object.entries(room.meta)) {
    console.log(`   - ${key}:`, entry);
  }
};

app.use("*", cors({ origin: "*" }));

app
  .post("/api/rooms", async (c) => {
    // Call to MAF Platform to create a new room
    const room = await server.rooms.create({
      // Meta entries are a way to pass information from your server to your
      // MAF app. If public, they can also be read by clients.
      meta: {
        privateMeta: "this is a private meta value from the server!",
        publicMeta: {
          visibility: "PUBLIC",
          value: "hello from the server!",
        },
      },
    });

    console.log("Created room:");
    printRoomInfo(room);

    return c.json({ type: "success", data: room });
  })
  .get("/api/rooms", async (c) => {
    // Call to MAF Platform to list rooms
    const rooms = await server.rooms.list();

    console.log("Listed rooms:");
    for (const room of rooms) printRoomInfo(room);

    return c.json(rooms.map((room) => ({ id: room.id })));
  })
  .post("/api/rooms/:roomId/token", async (c) => {
    const { roomId } = c.req.param();
    const room = await server.room({ tag: "id", id: roomId });
    if (!room) {
      return c.json(
        { type: "error", message: `Room with ID ${roomId} not found` },
        404
      );
    }

    // Sign a short-lived token for connecting to the room. This token is your
    // server confirming to the MAF servers that the user is (1) allowed to join
    // the room and (2) is who they say they are (their identity should be put
    // in the token).
    //
    // NOTE: In a real application, you would want to authenticate the user
    // before issuing a token, and you would want to ensure that the user is
    // authorized to join the room they are requesting a token for.
    const token = await room.sign({
      sub: "example-user-id",
      customData: "hello from server.ts!",
    });

    return c.json({ type: "success", data: { token } });
  })
  .onError((err, c) => {
    console.error("Unexpected error:", err);
    return c.json({ type: "error", message: "Internal server error" }, 500);
  });

export default {
  fetch: app.fetch,
  port: 8080,
};
