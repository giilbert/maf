import { MafClient } from "@usemaf/client";

const client = new MafClient({
  server: "dev",
});

type JsonData =
  | {
      type: "success";
      data: {
        id: string;
        secret: string;
      };
    }
  | {
      type: "error";
    };

async function run() {
  console.log("Creating room...");

  const res = await fetch("http://localhost:8080/api/rooms", {
    method: "POST",
  });
  const json = (await res.json()) as JsonData;

  if (json.type === "error") {
    throw new Error("Failed to create room");
  }

  const { id, secret } = json.data;
  console.log("Joining room with ID:", id, "and secret:", secret);

  await client.connect({ type: "room", id, secret });

  console.log("Connected to room:", id);

  while (true) {
    const result = await client.rpc<number>("increment_counter", 2);
    console.log("incremented counter! new value: ", result);
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
}

run();
