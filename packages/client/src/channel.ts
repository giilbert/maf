import Emittery from "emittery";
import type { CobbleUntypedBaseClient } from "./client";

export interface ChannelEvents<T> {
  message: T;
}

export class Channel<T> extends Emittery<ChannelEvents<T>> {
  private readonly client: CobbleUntypedBaseClient;
  private readonly name: string;

  constructor(client: CobbleUntypedBaseClient, name: string) {
    super();

    this.client = client;
    this.name = name;
  }

  public send(message: T) {
    this.client.send({
      type: "ChannelSend",
      data: {
        channel: this.name,
        data: message,
      },
    });
  }
}
