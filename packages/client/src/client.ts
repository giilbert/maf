import Emittery from "emittery";

export interface MafClientOptions {
  url: URL | string;
  app?: string;
}

export interface MafClientEvents {
  ready: undefined;
}

export class MafClient extends Emittery<MafClientEvents> {
  public readonly url: URL;

  private sessionInfo?: {
    id: string;
  };

  constructor(options: MafClientOptions) {
    super();

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
      ws.addEventListener("open", resolve, { once: true });
      ws.addEventListener("error", reject, { once: true });
    });

    ws.send(
      JSON.stringify({
        type: "handshake",
        data: {
          auth: {
            username: "hello",
            session: "12345",
          },
        },
      })
    );

    const handshakeResponse = await new Promise((resolve, reject) => {
      ws.addEventListener("message", (event) => {
        const data = JSON.parse(event.data);
        if (data.type === "handshake") {
          resolve(data);
        }
      });

      ws.addEventListener("error", reject, { once: true });
    });

    await this.emit("ready");
  }
}
