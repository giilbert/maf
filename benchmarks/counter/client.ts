import { MafClient } from "@usemaf/client";

const client = new MafClient({ server: "dev" });

await client.connect();
console.log("maf client connected!");

const ROUNDs = 10_000;

const times: number[] = [];
for (let i = 0; i < ROUNDs; i++) {
  const start = performance.now();
  await client.rpc("increment_counter", 1);
  const end = performance.now();
  console.log(`rpc call ${i + 1} took ${(end - start).toFixed(1)} ms`);
  times.push(end - start);

  if ((i + 1) % 100 === 0) {
    console.log(`completed ${i + 1} rounds`);
    const avgTime = times.reduce((acc, curr) => acc + curr, 0) / times.length;
    console.log(`average time per RPC: ${avgTime.toFixed(2)} ms`);
  }
}
