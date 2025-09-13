declare module "wasi:io/poll@0.2.6" {
  export function poll(pollables: Pollable[]): Uint32Array;

  export interface Pollable {
    ready(): boolean;
    block(): void;
    [Symbol.dispose](): void;
  }
}

declare module "maf:bindings/bindings" {
  import { Pollable } from "wasi:io/poll@0.2.6";

  interface UserMeta {
    id: [bigint, bigint];
  }

  interface User {
    meta(): void;
    listenMessage(): FutureMessage;
    send(message: Message): void;
    [Symbol.dispose](): void;
  }

  export type ListenError =
    | {
        tag: "closed";
      }
    | {
        tag: "already-listening";
      }
    | {
        tag: "not-ready";
      };

  export interface FutureUser {
    subscribe(): Pollable;
    get(): User;
    [Symbol.dispose](): void;
  }

  type Message =
    | {
        tag: "text";
        val: string;
      }
    | {
        tag: "binary";
        val: Uint8Array;
      };

  export interface FutureMessage {
    subscribe(): Pollable;
    get(): Message;
    [Symbol.dispose](): void;
  }

  export function listenUser(): FutureUser;
}
