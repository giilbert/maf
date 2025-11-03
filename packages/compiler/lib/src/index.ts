import { ListenError, listenUser, Message, User } from "maf:bindings/bindings";
import { reactor } from "./io-reactor";
import { RpcBuilder, RpcBuilderWithInput } from "./rpc";
import { type ZodSchema } from "zod";
import { debug, DebugLevel } from "./debug";

export class App {
  public _rpcMethods: Record<string, RpcBuilderWithInput<ZodSchema>> = {};

  constructor(public options: { debug?: DebugLevel } = {}) {
    debug.level = options.debug || "none";
  }

  public rpc(name: string): RpcBuilder {
    return new RpcBuilder(this, name);
  }

  private async handleUser(user: User) {
    const meta = user.meta();
    debug.trace("app", "user connected. meta:", meta);

    const messageListener = user.listenMessage();

    while (true) {
      const message = await reactor.await<Message, ListenError>(
        messageListener,
        "next-message"
      );

      if (message.type === "error") {
        if (message.error.tag === "closed") break;
        debug.trace("app", "user rx error:", message.error);
        break;
      }

      if (message.value.tag === "text") {
        const rx: RxPacket = JSON.parse(message.value.val);
        debug.trace("app", "user rx", rx);

        if (rx.type === "TypedRpcCall") {
          const rpcMethod = this._rpcMethods[rx.data.method];
          if (!rpcMethod) {
            // who cares about error handling
            throw new Error("RPC method not found: " + rx.data.method);
          }

          // FIXME: actually handle errors properly blaAH
          try {
            const result = rpcMethod.call(rx.data.params);
            user.send({
              tag: "text",
              val: JSON.stringify({
                type: "TypedRpcResponse",
                data: {
                  id: rx.data.id,
                  result: await result,
                },
              }),
            });
          } catch (err) {
            console.log("Error handling RPC call:", err);
            console.log(err.payload);
            throw err;
          }
        }
      }
    }
  }

  private async runUsers() {
    const listener = listenUser();

    while (true) {
      const user = await reactor.await<User, ListenError>(
        listener,
        "next-user"
      );

      if (user.type !== "ready") continue;

      this.handleUser(user.value).catch((err) => {
        console.error("Error handling user:", err);
      });
    }
  }

  run() {
    this.runUsers().finally(() => {
      console.log("user listener down");
    });

    reactor.run();
  }
}
