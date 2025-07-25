import { MafServiceClient } from ".";
import { PlatformApiError } from "./error";

export interface Room {
  id: string;
  key: string;
  secret: string;
}

export class Rooms {
  constructor(private client: MafServiceClient) {}

  /**
   * **GET** `/api/v1/apps/{app}/rooms`
   */
  async list() {
    const url = new URL(
      `api/v1/apps/${this.client.app}/rooms`,
      this.client.serverBaseUrl
    );

    const response = await fetch(url, {
      method: "GET",
      headers: { Authorization: this.client.authorization },
    });

    if (!response.ok) {
      throw new PlatformApiError(
        `Failed to list rooms: ${response.statusText}`,
        await response.json()
      );
    }

    return (await response.json()) as Room[];
  }

  /**
   * **POST** `/api/v1/apps/{app}/rooms`
   */
  async create() {
    const url = new URL(
      `api/v1/apps/${this.client.app}/rooms`,
      this.client.serverBaseUrl
    );

    const response = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: this.client.authorization,
      },
      body: JSON.stringify({}),
    });

    if (!response.ok) {
      throw new PlatformApiError(
        `Failed to create room: ${response.statusText}`,
        await response.json()
      );
    }

    const data = (await response.json()) as Room;

    return data;
  }
}
