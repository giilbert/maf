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
      const message = await new Promise<
        | {
            type: "message";
            data: Message;
          }
        | {
            type: "error";
            data: ListenError;
          }
      >((resolve) => {
        reactor.addPollable(
          messageListener.subscribe(),
          () => {
            try {
              resolve({ type: "message", data: messageListener.get() });
            } catch (err) {
              const typed = err as { payload: ListenError };
              resolve({
                type: "error",
                data: typed.payload,
              });
            }
          },
          "next-message"
        );
      });

      if (message.type === "error") {
        if (message.data.tag === "closed") break;

        console.log("user message error:", message.data);
        break;
      }

      console.log("user message:", message);
    }
  }

  private async runUsers() {
    const listener = listenUser();

    while (true) {
      const user = await new Promise<User>((resolve) => {
        reactor.addPollable(
          listener.subscribe(),
          () => resolve(listener.get()),
          "next-user"
        );
      });

      this.handleUser(user).catch((err) => {
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
