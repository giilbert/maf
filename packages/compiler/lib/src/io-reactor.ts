import { poll, Pollable } from "wasi:io/poll@0.2.4";

interface PollableWithHandler {
  pollable: Pollable;
  handler: (() => void) | null;
}

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
    console.log("---------- pollables ----------");
    for (const [index] of this.pollables.entries()) {
      const name = this.names[index];
      console.log(` - ${index}: ${name}`);
    }
  }

  public run() {
    if (this.pollables.length === 0) {
      console.log("no pollables, exiting...");
      return;
    }

    // this.printPollables();
    // console.log(`io: sleeping (${this.pollables.length} pollables)`);
    const readyIndices = poll(this.pollables);
    // console.log(`io: reactor woke up (${readyIndices.length} ready)`);

    for (const index of readyIndices) {
      // console.log(`io: waking up ${index} (${this.names[index]})`);
      const handler = this.handlers[index];
      handler();
    }

    // console.log("io: removing pollables");

    readyIndices.reverse();
    // TODO: are we sure the indices are in decreasing order?
    for (const index of readyIndices) {
      this.removePollable(index);
    }

    // this.printPollables();

    queueMicrotask(() => {
      this.run();
    });
  }
}

export const reactor = new IoReactor();
