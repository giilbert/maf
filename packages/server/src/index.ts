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

class Rooms {
  constructor(private client: MafServiceClient) {}

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
    });

    if (!response.ok) {
      throw new Error(`Failed to create room: ${response.statusText}`);
    }

    const data = (await response.json()) as {
      id: string;
      secret: string;
    };

    return data;
  }
}
