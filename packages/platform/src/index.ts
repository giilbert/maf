import { AnyRoomId, Room, Rooms } from "./rooms";
import { DEFAULT_DEV_SERVER_URL, type MafServerOptions } from "@usemaf/client";

export interface MafServiceClientOptions {
  server: MafServerOptions;

  clientId: string;
  clientSecret: string;
}

/**
 * Client for interacting with the MAF Platform API.
 *
 * @example
 * const client = new MafServiceClient({
 *   server: "dev",
 *   clientId: process.env.MAF_CLIENT_ID!,
 *   clientSecret: process.env.MAF_CLIENT_SECRET!,
 * });
 *
 * // List rooms
 * const rooms = await client.rooms.list();
 * console.log(rooms);
 */
export class MafServiceClient {
  public readonly authorization: string;
  public readonly server: MafServerOptions;

  public readonly rooms: Rooms;

  constructor(options: MafServiceClientOptions) {
    this.server = options.server;
    this.authorization = `Basic ${btoa(
      `${options.clientId}:${options.clientSecret}`
    )}`;

    this.rooms = new Rooms(this);
  }

  /**
   * The app identifier for the current server.
   */
  public get app() {
    // In development use a default "_/_" app for parity with Platform APIs
    if (this.server === "dev" || this.server.type === "dev") {
      return "_/_";
    } else if (this.server.type === "platform") {
      return this.server.app;
    }
  }

  /**
   * Finds and returns a {@link Room} by its ID or key.
   *
   * @param query Options to query for the room.
   * @returns An instance of the room, if it exists.
   * @throws {PlatformApiError} If the room could not be found or another error
   * occured.
   */
  public async room(query: AnyRoomId) {
    const room = new Room(this);
    await room.init(query);
    return room;
  }

  /**
   * Makes an authenticated fetch request to the MAF Platform APIs, including
   * the proper headers and URL base.
   *
   * @param path The API path to request.
   * @param init Fetch options.
   * @returns The fetch response.
   */
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
