import { MafClient } from "@maf/client";

const client = new MafClient({
  url: "http://localhost:3000",
  app: "maf/example-simple",
});

async function run() {
  await client.connect();

  const channel = client.channel<string>("example-channel");

  while (true) {
    const message = channel.send("hello");
    await new Promise((resolve) => {
      setTimeout(resolve, 1000);
    });
  }
}

client.on("ready", (session) => {
  console.log("client is ready!", session);
});

run();
