import { Rooms } from "./rooms";

export interface MafServiceClientOptions {
  url: URL | string;
  app: string;

  clientId: string;
  clientSecret: string;
}

export class MafServiceClient {
  public readonly authorization: string;
  public readonly serverBaseUrl: URL;
  public readonly app: string;

  public readonly rooms: Rooms;

  constructor(options: MafServiceClientOptions) {
    const url =
      typeof options.url === "string" ? new URL(options.url) : options.url;

    this.serverBaseUrl = url;
    this.authorization = `Basic ${btoa(
      `${options.clientId}:${options.clientSecret}`
    )}`;

    this.rooms = new Rooms(this);
    this.app = options.app;
  }
}
