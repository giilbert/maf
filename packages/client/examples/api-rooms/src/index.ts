import { MafClient } from "@maf/client";

const client = new MafClient({
  url: "http://localhost:3000",
  app: "gilbert/test-2",
});

async function run() {
  const res = await fetch("/api/rooms", { method: "POST" });
  if (!res.ok) {
    throw new Error(`Failed to create room: ${res.statusText}`);
  }
}

run();
