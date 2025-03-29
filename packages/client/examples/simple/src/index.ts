import { MafClient } from "@maf/client";

const client = new MafClient({
  url: "http://localhost:3000",
  app: "maf/example-simple",
});

async function run() {
  await client.connect();
}

client.on("ready", () => {
  console.log("Client is ready!");
});

run();
