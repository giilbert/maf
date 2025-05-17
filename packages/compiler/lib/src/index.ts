import { listenUser } from "maf:bindings/bindings";
import "./modules";
import { reactor } from "./io-reactor";

class App {
  constructor() {}
}

class AppBuilder {
  public rpcMethods: Record<string, () => void> = {};

  constructor() {}

  public rpc(method: string, handler: () => void) {
    this.rpcMethods[method] = handler;
    return this;
  }

  run() {
    const users = listenUser();
    reactor.addPollable(users.subscribe(), () => {
      const user = users.get();
      console.log("user:", user);
    });

    reactor.run();
  }
}

export const app = () => {
  return new AppBuilder();
};

export const test = () => {
  console.log("test");
};
