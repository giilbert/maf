import Emittery from "emittery";
import type { MafClient } from "./client";

export interface StoreOptions<T> {
  default?: T;
}

export interface StoreEvents<T> {
  change: T;
}

export class Store<T> extends Emittery<StoreEvents<T>> {
  private readonly client: MafClient;
  private readonly name: string;

  private _hasInit: boolean = false;
  private _data: T | null = null;

  /**
   * This promises is resolved when the store is first populated. If not
   * awaited, the data may invalidly contain null when it is populated on the
   * server.
   *
   * ## Usage
   * ```typescript
   * const store = maf.store<string[]>("names");
   * await store.init;
   * ```
   */
  public readonly init: Promise<void>;

  constructor(client: MafClient, name: string, options?: StoreOptions<T>) {
    super();

    const storeInit = options?.default ?? null;

    this.client = client;
    this.name = name;

    if (storeInit) {
      this._data = storeInit;
      this._hasInit = true;
    }

    this.on("change", (data) => {
      this._data = data;
    });

    this.init = new Promise((resolve) => {
      if (this._hasInit) return resolve();
      this.once("change").then(() => resolve());
    });
  }

  /**
   * Gets the data currently inside the store. This getter is guaranteed to
   * result in a non-null `T`.
   *
   * If the store has not been initialized (see `this.init`), this getter method
   * will error. `this.get` is the fallible version of this, returning null
   * if the store has not been initialized.
   */
  get data(): T {
    if (!this._hasInit)
      throw new Error("Store has not been initialized with data.");
    return this._data as T;
  }

  get(): T | null {
    return this._data;
  }
}
