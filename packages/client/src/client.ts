export interface MafClientOptions {
  url: URL | string;
  app?: string;
}

export class MafClient {
  public readonly url: URL;

  constructor(options: MafClientOptions) {
    const url =
      typeof options.url === "string" ? new URL(options.url) : options.url;

    if (options.app) {
      url.pathname = `@/${options.app}`;
    }

    console.log(url);

    this.url = url;
  }

  async connect() {
    const connectionUrl = new URL(this.url);
    connectionUrl.pathname += "/connect";

    const ws = new WebSocket(connectionUrl);

    await new Promise((resolve, reject) => {
      ws.addEventListener("open", resolve);
      ws.addEventListener("error", reject);
    });
  }
}
