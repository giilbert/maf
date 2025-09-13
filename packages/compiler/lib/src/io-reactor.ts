import { poll, Pollable } from "wasi:io/poll@0.2.6";
import { debug } from "./debug";

type AwaitResult<T, E = void> =
  | { type: "ready"; value: T }
  | { type: "error"; error: E };

export class IoReactor {
  private pollables: Pollable[] = [];
  private handlers: (() => void)[] = [];
  private names: string[] = [];

  constructor() {}

  public addPollable(pollable: Pollable, handler: () => void, name?: string) {
    this.pollables.push(pollable);
    this.handlers.push(handler);
    this.names.push(name || "<unknown>");
  }

  public await<T, E = void>(
    pollableWithGet: {
      subscribe: () => Pollable;
      get: () => T;
    },
    name?: string
  ) {
    return new Promise<AwaitResult<T, E>>((resolve) => {
      this.addPollable(
        pollableWithGet.subscribe(),
        () => {
          try {
            resolve({ type: "ready", value: pollableWithGet.get() });
          } catch (err) {
            resolve({ type: "error", error: err as E });
          }
        },
        name || "<unknown>"
      );
    });
  }

  /**
   * Removes a pollable by index, replacing it with the last one in the array.
   */
  private removePollable(index: number) {
    // If the index is at the end of the array, just pop it off.
    if (index === this.pollables.length - 1) {
      this.pollables.pop();
      this.handlers.pop();
      this.names.pop();
      return;
    }

    const lastPollable = this.pollables.pop();
    const lastHandler = this.handlers.pop();
    const lastName = this.names.pop();
    if (!lastPollable || !lastHandler || !lastName) {
      throw new Error("Failed to pop last pollable or handler");
    }

    const oldPollable = this.pollables[index];
    oldPollable[Symbol.dispose]();

    // Replace the pollable and handler at the index with the last one.
    this.pollables[index] = lastPollable;
    // TODO: do we need to dispose of the pollable via Symbol.dispose?
    this.handlers[index] = lastHandler;
    this.names[index] = lastName;
  }

  private printPollables() {
    debug.trace("io", "---------- pollables ----------");
    for (const [index] of this.pollables.entries()) {
      const name = this.names[index];
      debug.trace("io", ` - ${index}: ${name}`);
    }
  }

  public run() {
    // Keep advancing the event loop until there is a pollable to wait on.
    if (this.pollables.length === 0) {
      debug.trace("io", "io: requeue due to no pollables");
      queueMicrotask(() => this.run());
      return;
    }

    this.printPollables();
    queueMicrotask(() => this.sleep());
    this.printPollables();

    debug.trace("io", "io: run done. waiting for requeue");
  }

  private sleep() {
    debug.trace("io", `io: sleeping (${this.pollables.length} pollables)`);

    const readyIndices = poll(this.pollables);
    debug.trace("io", `io: reactor woke up (${readyIndices.length} ready)`);

    for (const index of readyIndices) {
      debug.trace("io", `io: waking up ${index} (${this.names[index]})`);
      const handler = this.handlers[index];
      handler();
    }

    debug.trace("io", "io: removing pollables");

    readyIndices.reverse();
    // TODO: are we sure the indices are in decreasing order?
    for (const index of readyIndices) {
      this.removePollable(index);
    }

    queueMicrotask(() => this.run());
  }
}

export const reactor = new IoReactor();
