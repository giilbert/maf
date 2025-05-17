declare module "wasi:io/poll@0.2.4" {
  export function poll(pollables: Pollable[]): Uint32Array;

  export interface Pollable {
    ready(): boolean;
    block(): void;
  }
}

declare module "maf:bindings/bindings" {
  import { Pollable } from "wasi:io/poll@0.2.4";

  interface User {
    meta(): void;
  }

  export interface FutureUser {
    subscribe(): Pollable;
    get(): User;
  }

  export function listenUser(): FutureUser;
}
