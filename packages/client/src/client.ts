import Emittery from "emittery";
import { Channel } from "./channel";
import { Store, StoreOptions } from "./store";
import { RxPacket, TxPacket } from "./packet";

export interface MafClientOptions {
  url: URL | string;
  app?: string;
}

export interface MafClientEvents {
  ready: SessionInfo;
  close: void;
}

export interface SessionInfo {
  id: string;
}

export type ConnectOptions =
  | {
      type: "default";
    }
  | {
      type: "room";
      id: string;
      secret: string;
    };

export class MafClient extends Emittery<MafClientEvents> {
  public readonly url: URL;

  private _sessionInfo?: SessionInfo;
  private _ws?: WebSocket;

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private _channels: Record<string, Channel<any>> = {};
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private _stores: Record<string, Store<any>> = {};
  private _storeData: Record<string, unknown> = {};

  private _rpcId = 0;
  private _rpcCalls: Map<number, (data: unknown) => void> = new Map();

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

  async connect(options: ConnectOptions = { type: "default" }) {
    const connectionUrl = new URL(this.url);
    if (options.type === "room") {
      // If the connection type is "room" (authenticated api request, etc.), add
      // the room ID to the path
      connectionUrl.pathname += `/${options.id}/connect`;
      connectionUrl.searchParams.set("secret", options.secret);
    } else if (options.type === "default") {
      // If the connection type is "default" (auto created room), use "default"
      connectionUrl.pathname += "/default/connect";
    } else {
      throw new Error("Invalid connection options");
    }

    const ws = new WebSocket(connectionUrl);
    this._ws = ws;

    await new Promise((resolve, reject) => {
      ws.addEventListener("open", resolve, { once: true });
      ws.addEventListener("error", reject, { once: true });
    });

    ws.send(
      JSON.stringify({
        type: "Handshake",
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
            if (type === "Handshake") resolve(data);
          },
          { once: true }
        );

        ws.addEventListener("error", reject, { once: true });
      }
    );

    this._sessionInfo = handshakeResponse;
    this.emit("ready", handshakeResponse);

    const handleMessage = (event: MessageEvent) => {
      if (typeof event.data === "string") {
        this.handleMessage(JSON.parse(event.data));
      } else {
        console.warn("Received non-string message:", event.data);
      }
    };
    ws.addEventListener("message", handleMessage);

    ws.addEventListener("close", () => {
      ws.removeEventListener("message", handleMessage);
      this.emit("close", undefined);
    });

    return handshakeResponse;
  }

  private async handleMessage(packet: RxPacket) {
    if (packet.type === "ChannelSend") {
      const { channel, data } = packet.data;
      this._channels[channel]?.emit("message", data);
    } else if (packet.type === "TypedRpcResponse") {
      const { id, result } = packet.data;
      this._rpcCalls.get(id)?.(result);
      this._rpcCalls.delete(id);
    } else if (packet.type === "ManyStoreUpdate") {
      for (const { store, data } of packet.data) {
        this._storeData[store] = data;
        this._stores[store]?.emit("change", data);
      }
    } else if (packet.type === "StoreUpdate") {
      const { store, data } = packet.data;
      this._storeData[store] = data;
      this._stores[store]?.emit("change", data);
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

  public rpc<T>(method: string, ...params: unknown[]) {
    const id = this._rpcId++;

    this.send({
      type: "TypedRpcCall",
      data: {
        method,
        id,
        params: params.length === 1 ? params[0] : params,
      },
    });

    return new Promise<T>((resolve, reject) => {
      // Arbitrary limit to prevent out-of-memory errors
      const MAX_RPC_CALLS = 5_000;

      if (this._rpcCalls.size > MAX_RPC_CALLS) {
        reject(
          new Error(`Maximum number of RPC calls exceeded (${MAX_RPC_CALLS})`)
        );
        return;
      }

      // TODO: handle timeout
      this._rpcCalls.set(id, (data) => {
        if (data instanceof Error) {
          reject(data);
        } else {
          resolve(data as T);
        }
      });
    });
  }

  public store<T>(name: string, options?: StoreOptions<T>) {
    const data = this._storeData[name];
    if (!this._stores[name])
      this._stores[name] = new Store(this, name, {
        default: data as T,
        ...options,
      });
    return this._stores[name] as Store<T>;
  }
}
