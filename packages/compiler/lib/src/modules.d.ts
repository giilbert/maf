declare module "wasi:io/poll@0.2.4" {
  export function poll(pollables: Pollable[]): Uint32Array;

  export interface Pollable {
    ready(): boolean;
    block(): void;
    [Symbol.dispose](): void;
  }
}

declare module "maf:bindings/bindings" {
  import { Pollable } from "wasi:io/poll@0.2.4";

  interface UserMeta {
    id: [bigint, bigint];
  }

  interface User {
    meta(): void;
    listenMessage(): FutureMessage;
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
        text: string;
      }
    | {
        tag: "binary";
        data: Uint8Array;
      };

  export interface FutureMessage {
    subscribe(): Pollable;
    get(): Message;
    [Symbol.dispose](): void;
  }

  export function listenUser(): FutureUser;
}
