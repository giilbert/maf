import { MafClient } from "@maf/client";

const client = new MafClient({
  url: "http://localhost:3000",
  app: "gilbert/test-2",
});

const store = client.store("CounterStore");

store.on("change", (data) => {
  console.log("Counter value changed:", data);
});

async function run() {
  await client.connect();

  console.log("Connected to the server!");

  while (true) {
    const _result = await client.rpc<number>("increment_counter", 2);
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
}

run();
