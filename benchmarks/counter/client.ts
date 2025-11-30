import { MafClient } from "@usemaf/client";

const client = new MafClient({ server: "dev" });

await client.connect();
console.log("maf client connected!");

const ROUNDs = 10_000;

const times = [];
for (let i = 0; i < ROUNDs; i++) {
  const start = performance.now();
  await client.rpc("increment_counter", 1);
  const end = performance.now();
  times.push(end - start);

  if ((i + 1) % 100 === 0) {
    console.log(`completed ${i + 1} rounds`);
    const avgTime = times.reduce((acc, curr) => acc + curr, 0) / times.length;
    console.log(`average time per RPC: ${avgTime.toFixed(2)} ms`);
  }
}
