# MAF API Rooms Example

This example demonstrates how to create and join rooms using server-side and MAF
Platform APIs to create rooms, and the `MafClient` to connect to those rooms.

## Running the Example

Please ensure that you have the following installed:
- Bun (https://bun.sh)
- pnpm (https://pnpm.io)
- MAF CLI (http://maf.gilbertz.me/docs/getting-started/quickstart)

1. Install dependencies:

```bash
pnpm i
```

2. Run the `platform` (`crates/maf/examples/platform`) example server with MAF
CLI:

```bash
cd crates/maf/examples/platform
maf run
```

3. Run the frontend and backend servers for the API Rooms example (in a separate
terminal):

```bash
cd packages/platform/examples/api-rooms
bun run server.ts
```


