/**
 * This is client-side code that demonstrates how to use a server to create
 * rooms via the MAF Platform API and then connect to those rooms using
 * {@link MafClient}.
 *
 * See `packages/platform/examples/api-rooms/server.ts` for the server-side
 * code that handles room creation requests.
 */

import { MafClient } from "@usemaf/client";

const client = new MafClient({
  // You would likely have a import.meta.env switch here to choose between
  // dev/prod servers.
  server: "dev",
  // Uncomment to use local platform server
  // server: {
  //   type: "platform",
  //   app: "gilbert/example-platform",
  //   url: "http://localhost:1147",
  // },
});

// I actually don't like builing frontends but here is a minimal element to
// display some server stuff on the page. - gilbert
const ui = document.getElementById("ui")!;
const roomList = document.getElementById("room-list")!;
const incrementCounter = document.getElementById("increment-counter")!;
incrementCounter!.setAttribute("disabled", "true");

async function createRoom() {
  // Create a new room by calling our server's API.
  //
  // The point of emphasis here is that YOU are able to write this server-side
  // code to manage rooms and connect people to them. In other words, you are
  // in control of things like:
  // - how rooms are provisions and how users are grouped together
  // - what authentication/authorization mechanisms are used
  // - ratelimiting, logging, monitoring, etc.
  //
  // See server.ts for the server implementation.
  console.log("Creating room by calling an API YOU WRITE...");
  const res = await fetch("http://localhost:8080/api/rooms", {
    method: "POST",
  });

  // The server responds with JSON indicating success or failure.
  type CreateRoomResponseBody =
    | { type: "success"; data: { id: string } }
    | { type: "error" }; //    ^^^^^^^^^^^^^^
  // The server returns the room ID here. This lets us (the client) connect to
  // the room we just created.

  const body: CreateRoomResponseBody = await res.json();
  if (body.type === "error") throw new Error("Failed to create room");

  return body.data;
}

async function createAndConnect() {
  const { id } = await createRoom();
  console.log("Joining room with ID:", id);
  ui.innerText = "Connecting to room...";

  await loadRoomList();
  await connect(id);
}

async function connect(id: string) {
  // If you do not turn on authentication, you can connect to rooms with just
  // the room ID.
  console.log("Connecting to room ID:", id);
  await client.connect({ type: "room", id });
  console.log("Connected to room:", id);
  ui.innerText = `Connected to room: ${id}\n\nCounter: <loading>`;

  // Once connected, we can use MAF APIs as normal. Here, we listen to a store
  // called "count" and update the UI whenever it changes.
  const count = client.store<number>("count");
  count.on("change", () => {
    ui.innerText = `Connected to room: ${id}\n\nCounter: ${count.get()}`;
  });

  // ... and a button that increments the counter via an RPC!
  incrementCounter!.removeAttribute("disabled");
  incrementCounter.onclick = async () => {
    const newValue = await client.rpc<number>("increment_counter", 1);
    console.log("incremented counter! new value: ", newValue);
  };
}

async function loadRoomList() {
  // Here is another example of calling our server to get a list of existing
  // rooms. This is just to demonstrate that you can build your own server-side
  // APIs that interact with rooms however you like.
  //
  // In a real application, you might not want to expose all room IDs like this.
  // Instead, you might route users to rooms based on some grouping that already
  // exists and makes sense for your application (e.g. in a chat app, you'll
  // want people in chat rooms routed to the same rooms).
  console.log("Loading room list from server...");

  const res = await fetch("http://localhost:8080/api/rooms");
  const rooms: { id: string }[] = await res.json();
  let text = `There ${
    rooms.length === 1 ? "is" : "are"
  } currently ${rooms.length} room${rooms.length !== 1 ? "s" : ""}:\n\n`;
  for (const room of rooms) {
    text += `- ID: ${room.id}\n`;
  }
  roomList.innerText = text;
}

ui.innerText = "Select an option on the left!";

loadRoomList().catch((e) => {
  roomList.innerText = "Failed to load room list. Check console.";
  console.error("Failed to load room list:", e);
});

// Weird UI stuff!!!

const startRoomButton = document.getElementById(
  "start-room"
) as HTMLButtonElement;
const joinRoomForm = document.getElementById(
  "join-room-form"
) as HTMLFormElement;
const joinRoomInput = document.getElementById(
  "join-room-id"
) as HTMLInputElement;
const joinRoomButton = document.getElementById(
  "join-room"
) as HTMLButtonElement;

startRoomButton.onclick = () => {
  startRoomButton.disabled = true;
  joinRoomForm.disabled = true;
  joinRoomInput.disabled = true;
  joinRoomButton.disabled = true;

  createAndConnect().catch((e) => {
    ui.innerText = "Failed to connect. Check console.";
    console.error("Failed to create/connect to room:", e);
  });
};

joinRoomForm.onsubmit = (e) => {
  e.preventDefault();

  startRoomButton.disabled = true;
  joinRoomForm.disabled = true;
  joinRoomInput.disabled = true;
  joinRoomButton.disabled = true;

  const roomId = joinRoomInput.value;
  connect(roomId).catch((e) => {
    ui.innerText = "Failed to connect. Check console.";
    console.error("Failed to connect to room:", e);
  });
};
