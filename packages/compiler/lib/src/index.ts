import { ListenError, listenUser, Message, User } from "maf:bindings/bindings";
import { reactor } from "./io-reactor";

class App {
  public rpcMethods: Record<string, () => void> = {};

  constructor() {}

  public rpc(method: string, handler: () => void) {
    this.rpcMethods[method] = handler;
    return this;
  }

  private async handleUser(user: User) {
    const meta = user.meta();
    console.log("user connected! meta:", meta);

    const messageListener = user.listenMessage();

    while (true) {
      const message = await reactor.await<Message, ListenError>(
        messageListener,
        "next-message"
      );

      if (message.type === "error") {
        if (message.error.tag === "closed") break;
        console.log("user message error:", message.error);
        break;
      }

      console.log("user message:", message);
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

export const app = () => {
  return new App();
};
