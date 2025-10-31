import { Rooms } from "./rooms";
import {
  DEFAULT_DEV_SERVER_URL,
  type CobbleServerOptions,
} from "@usecobble/client";

export interface CobbleServiceClientOptions {
  server: CobbleServerOptions;

  clientId: string;
  clientSecret: string;
}

export class CobbleServiceClient {
  public readonly authorization: string;
  public readonly server: CobbleServerOptions;

  public readonly rooms: Rooms;

  constructor(options: CobbleServiceClientOptions) {
    this.server = options.server;
    this.authorization = `Basic ${btoa(
      `${options.clientId}:${options.clientSecret}`
    )}`;

    this.rooms = new Rooms(this);
  }

  public get app() {
    // In development use a default "_/_" app for parity with Platform APIs
    if (this.server === "dev" || this.server.type === "dev") {
      return "_/_";
    } else if (this.server.type === "platform") {
      return this.server.app;
    }
  }

  public async fetch(path: string, init?: RequestInit): Promise<Response> {
    const url = new URL(
      path,
      this.server === "dev"
        ? DEFAULT_DEV_SERVER_URL
        : this.server.url || DEFAULT_DEV_SERVER_URL
    );

    const headers: HeadersInit = {
      "Content-Type": "application/json",
      Authorization: this.authorization,
      ...init?.headers,
    };

    const response = await fetch(url, {
      ...init,
      headers,
    });

    return response;
  }
}
