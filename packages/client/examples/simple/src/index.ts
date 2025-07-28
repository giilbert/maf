import { MafClient } from "@usemaf/client";

const client = new MafClient({
  server: "dev",
});

const store = client.store("CounterStore");

store.on("change", (data) => {
  console.log("Counter value changed:", data);
});

async function run() {
  await client.connect();

  console.log("Connected to the server!");

  while (true) {
    const _result = await client.rpc<number>("increment_counter", 1);
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
}

run();
