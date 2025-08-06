import { TypedMafClient } from "@usemaf/client";
import type { MafApp } from "./types";

declare module "@usemaf/client" {
  interface MafTypes {
    generated: MafApp;
  }
}

const client = new TypedMafClient({
  server: "dev",
});

const store = client.store("count");

store.on("change", (data) => {
  console.log("Counter value changed:", data);
});

async function run() {
  await client.connect();

  console.log("Connected to the server!");

  while (true) {
    const _result = await client.rpc("increment_counter", 1);
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
}

run();
