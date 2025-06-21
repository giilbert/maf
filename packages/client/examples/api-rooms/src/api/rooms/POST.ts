import type { Request, Response } from "express";
import { MafServiceClient } from "@maf/server";

const server = new MafServiceClient({
  url: "http://localhost:3000",
  // Replace with your actual app name
  app: "gilbert/example-basic",
  // Fake credentials for testing purposes
  // In a real application, you would use actual credentials
  clientId: "test-client-id",
  clientSecret: "secret",
});

export default async (_req: Request, res: Response) => {
  try {
    const data = await server.rooms.create();
    return res.status(200).send({ type: "success", data });
  } catch (e) {
    console.error("Failed to create room:", e);
    return res.status(500).send({ type: "error" });
  }
};
