import { poll, Pollable } from "wasi:io/poll@0.2.4";

interface PollableWithHandler {
  pollable: Pollable;
  handler: (() => void) | null;
}

export class IoReactor {
  private pollables: Pollable[] = [];
  private handlers: (() => void)[] = [];

  constructor() {}

  public addPollable(pollable: Pollable, handler: () => void) {
    this.pollables.push(pollable);
    this.handlers.push(handler);
  }

  public run() {
    console.log("running io reactor..");

    while (true) {
      console.log("sleeping...");
      const readyIndices = poll(this.pollables);
      console.log("ready:", readyIndices);

      for (const index of readyIndices) {
        const handler = this.handlers[index];
        handler();
      }
    }
  }
}

export const reactor = new IoReactor();
