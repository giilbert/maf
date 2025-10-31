import { App } from "cobble";

const app = new App();

app.rpc("increment_counter").handler((ctx) => {
  console.log("RPC increment_counter called with params:", ctx.params);
});

export { app };
