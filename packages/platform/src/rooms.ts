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

    return (await response.json()) as Room[];
  }

  /**
   * **POST** `/api/v1/apps/{app}/rooms`
   */
  async create(options: { key?: string } = {}) {
    const response = await this.client.fetch(
      `/api/v1/apps/${this.client.app}/rooms`,
      {
        method: "POST",
        body: JSON.stringify(options),
      }
    );

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
