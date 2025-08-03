import Emittery from "emittery";
import type { MafBaseClient } from "./client";

export interface ChannelEvents<T> {
  message: T;
}

export class Channel<T> extends Emittery<ChannelEvents<T>> {
  private readonly client: MafBaseClient;
  private readonly name: string;

  constructor(client: MafBaseClient, name: string) {
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
