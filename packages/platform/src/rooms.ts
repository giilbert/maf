import { MafServiceClient } from ".";
import { PlatformApiError } from "./error";

// Various ways to identify a room.
export type AnyRoomId = { tag: "id"; id: string } | { tag: "key"; key: string };

const anyRoomIdToQueryParams = (identifier: AnyRoomId) => {
  switch (identifier.tag) {
    case "id":
      return `by_id=${encodeURIComponent(identifier.id)}`;
    case "key":
      return `by_key=${encodeURIComponent(identifier.key)}`;
  }
};

interface RoomInit {
  id: string;
  key: string;
  secret: string;
  meta: Record<string, unknown>;
}

/**
 * Represents a room within the MAF Platform. This class contains various
 * methods and properties for managing and interacting with a room.
 *
 * This class should be initialized via the `init` method if not provided with
 * initial data. If the room data is already known (e.g., from room creation),
 * it can be passed directly to the constructor. Trying to access properties
 * before initialization will result in an error.
 */
export class Room {
  private _data?: RoomInit;

  constructor(
    private client: MafServiceClient,
    init?: RoomInit
  ) {
    if (init) this._data = init;
  }

  public async init(identifier: AnyRoomId) {
    // Avoid re-fetching if already initialized.
    //
    // TODO: This is valid if the room's data cannot change, otherwise we need
    // another mechanism to refresh.
    if (this._data) return;

    const response = await this.client.fetch(
      `/api/v1/apps/${this.client.app}/rooms?${anyRoomIdToQueryParams(identifier)}`
    );

    if (!response.ok) {
      throw new PlatformApiError(
        `Failed to fetch room: ${response.statusText}`,
        await response.json()
      );
    }

    const data = (await response.json()) as RoomInit;

    this._data = data;
  }

  /**
   * A UUIDv4 identifier for the room.
   */
  get id() {
    if (!this._data) throw new Error("Room not initialized");
    return this._data.id;
  }

  /**
   * Another identifier for the room. Usually, this is a more developer or
   * user-friendly string compared to the room ID.
   */
  get key() {
    if (!this._data) throw new Error("Room not initialized");
    return this._data.key;
  }

  /**
   * A secret used to sign and verify JWT payloads. **DO NOT** expose this
   * secret in client-side code.
   */
  get secret() {
    if (!this._data) throw new Error("Room not initialized");
    return this._data.secret;
  }

  /**
   * A developer-defined metadata object associated with the room.
   */
  get meta() {
    if (!this._data) throw new Error("Room not initialized");
    return this._data.meta;
  }

  toJSON() {
    if (!this._data) throw new Error("Room not initialized");
    return {
      id: this._data.id,
      key: this._data.key,
    };
  }
}

export interface CreateRoomOptions {
  /**
   * An optional developer-defined key for the room. If not provided, the
   * platform will generate a random key.
   */
  key?: string;

  /**
   * Initial meta entries for the room.
   *
   * Each entry can either be a simple string value or an object with a `value`
   * and optional `visibility` property. If a simple string is provided, it is
   * treated as a private meta entry.
   *
   * See https://maf.gilbertz.me/docs/build/meta for more information.
   */
  meta?: Record<
    string,
    | {
        visibility?: "PUBLIC" | "PRIVATE";
        value: unknown;
      }
    | unknown
  >;
}

/**
 * API client for managing rooms within the MAF Platform.
 */
export class Rooms {
  constructor(private client: MafServiceClient) {}

  /**
   * **GET** `/api/v1/apps/{app}/rooms`.
   *
   * Lists all rooms for the current app.
   */
  async list() {
    const response = await this.client.fetch(
      `/api/v1/apps/${this.client.app}/rooms`,
      { method: "GET" }
    );

    if (!response.ok) {
      throw new PlatformApiError(
        `Failed to list rooms: ${response.statusText}`,
        await response.json()
      );
    }

    return ((await response.json()) as RoomInit[]).map(
      (data) => new Room(this.client, data)
    );
  }

  /**
   * **POST** `/api/v1/apps/{app}/rooms`
   *
   * Creates a new room for the current app. The app should be configured to use
   * API-managed rooms.
   */
  async create(options: CreateRoomOptions = {}) {
    // Fix options.meta to match expected format
    if (options.meta) {
      const fixedMeta: Record<
        string,
        { visibility: "PUBLIC" | "PRIVATE"; value: unknown }
      > = {};

      for (const [key, entry] of Object.entries(options.meta)) {
        if (typeof entry === "string") {
          fixedMeta[key] = { visibility: "PRIVATE", value: entry };
        } else if (
          typeof entry === "object" &&
          entry &&
          "value" in entry &&
          "visibility" in entry &&
          (entry.visibility === "PUBLIC" || entry.visibility === "PRIVATE")
        ) {
          fixedMeta[key] = {
            visibility: entry.visibility ?? "PRIVATE",
            value: entry.value,
          };
        }
      }

      options.meta = fixedMeta;
    }

    const response = await this.client.fetch(
      `/api/v1/apps/${this.client.app}/rooms`,
      {
        method: "POST",
        body: JSON.stringify(options),
      }
    );

    if (!response.ok) {
      console.log(response);
      throw new PlatformApiError(
        `Failed to create room: ${response.statusText}`,
        await response.json()
      );
    }

    const data = (await response.json()) as RoomInit;
    return new Room(this.client, data);
  }
}
