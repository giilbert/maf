import { MafClient } from "@maf/client";

const client = new MafClient({
  url: "http://localhost:3000",
  app: "gilbert/test-2",
});

async function run() {
  await client.connect();

  console.log("client connected!");

  while (true) {
    const result = await client.rpc<number>("test", 2);
    console.log("client rpc result", result);
    // console.log("client rpc test");
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
}

run();
