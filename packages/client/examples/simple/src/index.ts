import { MafClient } from "@maf/client";

const client = new MafClient({
  url: "http://localhost:3000",
  app: "gilbert/test",
});

async function run() {
  await client.connect();

  const channel = client.channel<string>("hello");

  channel.on("message", (message) => {
    console.log("received message:", message);
  });

  while (true) {
    channel.send("hello");
    await new Promise((resolve) => {
      setTimeout(resolve, 1000);
    });
  }
}

client.on("ready", (session) => {
  console.log("client is ready!", session);
});

run();
