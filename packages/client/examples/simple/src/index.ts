import { TypedCobbleClient } from "@usecobble/client";
import type { CobbleApp } from "./types";

declare module "@usecobble/client" {
  interface CobbleTypes {
    generated: CobbleApp;
  }
}

const client = new TypedCobbleClient({
  server: "dev",
});

const store = client.store("count");

store.on("change", (data) => {
  console.log("Counter value changed:", data);
});

const storeTwo = client.store("count_times_two");
storeTwo.on("change", (data) => {
  console.log("Count times two value changed:", data);
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
