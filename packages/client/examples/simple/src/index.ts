import { MafClient } from "../../../src/client";

const client = new MafClient({
  url: "http://localhost:3000",
  app: "maf/example-simple",
});

async function run() {
  console.log("trying to connect..");
  await client.connect();
  console.log("connected!");
}

run();
