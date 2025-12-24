import Emittery from "emittery";
import type { MafUntypedBaseClient } from "./client";

export interface ChannelEvents<T> {
  message: T;
}

/**
 * A channel for sending and receiving messages to/from a MAF server.
 *
 * @example
 * const maf = new MafClient(...);
 *
 * const messages = maf.channel<string>("messages"); // returns Channel<string>
 * //                           ^ specify the message type
 *
 * // Subscribe to incoming messages
 * messages.on("message", (msg) => {
 *   console.log("Received message:", msg);
 * });
 *
 * // Send a message
 * messages.send("Hello, server!");
 *
 * @see https://maf.gilbertz.me/docs/build/channel
 */
export class Channel<T> extends Emittery<ChannelEvents<T>> {
  private readonly client: MafUntypedBaseClient;
  private readonly name: string;

  constructor(client: MafUntypedBaseClient, name: string) {
    super();

    this.client = client;
    this.name = name;
  }

  /**
   * Send a message to the channel.
   * @param message The message to send.
   */
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
