import Emittery from "emittery";
import type { MafUntypedBaseClient } from "./client";

export interface StoreOptions<T> {
  /**
   * Initial default value for the store. This is useful for ensuring that
   * the store has a value before it is populated from the server.
   *
   * @default null
   */
  default?: T;
  /**
   * A predicate function to determine whether the store has been initialized
   * with valid data. This is useful for stores where `null` is a valid value,
   * and you want to distinguish between "not yet initialized" and "initialized
   * with null".
   * @param value The current value of the store.
   * @returns True if the store should be considered initialized.
   */
  hasInit?: (value: T | null) => boolean;
}

/**
 * Events emitted by the {@link Store} class.
 */
export interface StoreEvents<T> {
  change: T;
}

/**
 * A store represents a piece of state that is synchronized between the client
 * and the server. Stores can be subscribed to, and will emit events when their
 * data changes.
 *
 * @example
 * const maf = new MafClient({ server: "dev" });
 * await maf.connect();
 *
 * const count = maf.store<number>("count", { default: 0 });
 *
 * // Subscribe to changes in the store
 * count.on("change", (newCount) => {
 *   console.log("Count changed to", newCount);
 * });
 *
 * // or, access the current value directly
 * const getCurrentCountTimesTwo = () => {
 *   return count.data * 2;
 * }
 *
 * @see https://maf.gilbertz.me/docs/build/store
 */
export class Store<T> extends Emittery<StoreEvents<T>> {
  private readonly client: MafUntypedBaseClient;
  private readonly name: string;

  private _hasInit: boolean = false;
  private _data: T | null = null;

  /**
   * This promises is resolved when the store is first populated. If not
   * awaited, the data may invalidly contain null when it is populated on the
   * server.
   *
   * @example
   * const store = maf.store<string[]>("names");
   * await store.init;
   */
  public readonly init: Promise<void>;

  constructor(
    client: MafUntypedBaseClient,
    name: string,
    options?: StoreOptions<T>
  ) {
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

      const unsubscribe = this.on("change", (data) => {
        if (options?.hasInit && !options.hasInit(data)) return;

        this._hasInit = true;
        unsubscribe();
        resolve();
      });
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

  /**
   * Gets the data currently inside the store, or null if the store has not
   * been initialized.
   *
   * @returns The data inside the store, or null if not initialized.
   */
  get(): T | null {
    return this._data;
  }

  /**
   * Indicates whether the store has been initialized with valid data.
   */
  get hasInit(): boolean {
    return this._hasInit;
  }
}
