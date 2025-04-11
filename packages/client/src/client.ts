import Emittery from "emittery";
import { Channel } from "./channel";

export interface MafClientOptions {
  url: URL | string;
  app?: string;
}

export interface MafClientEvents {
  ready: SessionInfo;
}

export interface SessionInfo {
  id: string;
}

export class MafClient extends Emittery<MafClientEvents> {
  public readonly url: URL;

  private _sessionInfo?: SessionInfo;
  private _ws?: WebSocket;

  private _channels: Record<string, Channel<any>> = {};

  public get ws() {
    if (!this._ws) throw new Error("WebSocket is not connected");
    return this._ws;
  }

  public get sessionInfo() {
    if (!this._sessionInfo) throw new Error("Session info is not available");
    return this._sessionInfo;
  }

  constructor(options: MafClientOptions) {
    super();

    const url =
      typeof options.url === "string" ? new URL(options.url) : options.url;

    if (options.app) {
      url.pathname = `@/${options.app}`;
    }

    this.url = url;
  }

  async connect() {
    const connectionUrl = new URL(this.url);
    connectionUrl.pathname += "/connect";

    const ws = new WebSocket(connectionUrl);
    this._ws = ws;

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

    const handshakeResponse = await new Promise<SessionInfo>(
      (resolve, reject) => {
        ws.addEventListener(
          "message",
          (event) => {
            const { data, type } = JSON.parse(event.data);
            if (type === "handshake") resolve(data);
          },
          { once: true }
        );

        ws.addEventListener("error", reject, { once: true });
      }
    );

    this._sessionInfo = handshakeResponse;
    this.emit("ready", handshakeResponse);

    ws.addEventListener("message", (event) => {
      if (typeof event.data === "string") {
        this.handleMessage(JSON.parse(event.data));
      } else {
        console.warn("Received non-string message:", event.data);
      }
    });

    return handshakeResponse;
  }

  private async handleMessage(packet: RxPacket) {
    if (packet.type === "ChannelSend") {
      const { channel, data } = packet.data;
      this._channels[channel]?.emit("message", data);
    }
  }

  public channel<T>(name: string) {
    if (!this._channels[name])
      this._channels[name] = new Channel<T>(this, name);
    return this._channels[name];
  }

  public send(message: TxPacket) {
    this.ws.send(JSON.stringify(message));
  }
}
