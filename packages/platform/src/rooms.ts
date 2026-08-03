import { MafServiceClient } from ".";
import { PlatformApiError } from "./error";
import { SignJWT } from "jose";

// Various ways to identify a room.
//
// TODO: change this since we got rid of room IDs vs keys in the API
export type AnyRoomId = { tag: "id"; id: string } | { tag: "key"; key: string };

const anyRoomIdToPath = (identifier: AnyRoomId) => {
  return identifier.tag === "id" ? identifier.id : identifier.key;
};

interface RoomInit {
  id: string;
  keys: string[];
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
    init?: RoomInit,
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
      `/api/v1/apps/${this.client.app}/rooms/${anyRoomIdToPath(identifier)}`,
    );

    if (!response.ok) {
      throw new PlatformApiError(
        `Failed to fetch room: ${response.statusText}`,
        await response.json(),
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
   * user-friendly string compared to the room ID. If the developer did not
   * provide a custom key during room creation, this will be the same as the
   * room ID.
   */
  get key() {
    if (!this._data) throw new Error("Room not initialized");
    // The first key is the ID, the second is the developer-defined key (if any)
    return this._data.keys.length > 1 ? this._data.keys[1] : this._data.keys[0];
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

  /**
   * Signs a JWT payload using the room's secret. This is used for
   * authenticating clients connecting to the room. The token returned by this
   * method should be included in the connection request to the MAF servers and
   * will expire in 1 minute.
   *
   * @param data The payload data to sign.
   */
  sign(data: { sub?: string; [key: string]: unknown }) {
    if (!this._data) throw new Error("Room not initialized");
    const jwt = new SignJWT(
      Object.fromEntries(Object.entries(data).filter(([k]) => k !== "sub")),
    )
      .setProtectedHeader({ alg: "HS256" })
      .setIssuedAt()
      .setAudience(this.id)
      .setExpirationTime("1m");

    if (data.sub) jwt.setSubject(data.sub);

    return jwt.sign(new TextEncoder().encode(this.secret));
  }

  toJSON() {
    if (!this._data) throw new Error("Room not initialized");
    return {
      id: this._data.id,
      key: this.key,
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
      { method: "GET" },
    );

    if (!response.ok) {
      throw new PlatformApiError(
        `Failed to list rooms: ${response.statusText}`,
        await response.json(),
      );
    }

    return ((await response.json()) as RoomInit[]).map(
      (data) => new Room(this.client, data),
    );
  }

  /**
   * **POST** `/api/v1/apps/{app}/rooms` or **PUT** `/api/v1/apps/{app}/rooms`.
   *
   * We abstract this into a single method since the API will handle both cases
   * with the same endpoint and request bodies.
   */
  async _createInner(method: "POST" | "PUT", options: CreateRoomOptions = {}) {
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
        method,
        body: JSON.stringify(options),
      },
    );

    if (!response.ok) {
      console.log(response);
      throw new PlatformApiError(
        `Failed to create room: ${response.statusText}`,
        await response.json(),
      );
    }

    const data = (await response.json()) as RoomInit;
    return new Room(this.client, data);
  }

  /**
   * **POST** `/api/v1/apps/{app}/rooms`
   *
   * Creates a new room for the current app. The app should be configured to use
   * API-managed rooms.
   */
  async create(options: CreateRoomOptions = {}) {
    return this._createInner("POST", options);
  }

  /**
   * **PUT** `/api/v1/apps/{app}/rooms`
   *
   * Creates a new room for the current app if the room key is not already in
   * use, or returns information about the existing room if it does exist. If
   * the room already exists, this will reset its auto-shutdown timer
   * atomically.
   */
  async fetch(options: CreateRoomOptions = {}) {
    return this._createInner("PUT", options);
  }
}
