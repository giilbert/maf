import { MafUntypedBaseClient, type MafClientOptions } from "./client";
import type { Store, StoreOptions } from "./store";

/**
 * The schema definition for a store during type generation.
 * @see https://maf.gilbertz.me/docs/build/type-generation
 */
export interface StoreDefinition<S> {
  name: string;
  select: S;
}

/**
 * The schema definition for an RPC during type generation.
 * @see https://maf.gilbertz.me/docs/build/type-generation
 */
export interface RpcDefinition<P, R extends unknown | unknown[]> {
  name: string;
  params: P;
  result: R;
}

/**
 * If type generation has not been run, this interface provides default types.
 */
interface DefaultMafTypes {
  generated: {
    /**
     * If `true`, this indicates that the generated types have been populated.
     */
    __isTyped: false;
    stores: Record<string, StoreDefinition<unknown>>;
    rpcs: Record<string, RpcDefinition<unknown, unknown>>;
  };
}

/**
 * This interface should be augmented by the user to provide their own types.
 *
 * @see https://maf.gilbertz.me/docs/build/type-generation
 *
 * @example
 * import type { MafTypes } from "@usemaf/client";
 *
 * declare module "@usemaf/client" {
 *   interface MafTypes {
 *     generated: MafApp;
 *   }
 * }
 */
export interface MafTypes extends DefaultMafTypes {}

export type TypedStores = MafTypes["generated"]["stores"];
export type StoreKeys = keyof TypedStores;
export type StoreSelect<K extends StoreKeys> = TypedStores[K]["select"];

export type TypedRpcs = MafTypes["generated"]["rpcs"];
export type RpcKeys = keyof TypedRpcs;
export type RpcParams<K extends RpcKeys> =
  TypedRpcs[K]["params"] extends unknown[]
    ? TypedRpcs[K]["params"]
    : [TypedRpcs[K]["params"]];

export class TypedMafClient extends MafUntypedBaseClient {
  constructor(options: MafClientOptions) {
    super(options);
  }

  /**
   * Get a store by name.
   *
   * This method is typed based on the generated store definitions.
   *
   * @param name The name of the store as defined on the server. This should be
   * available in the generated types.
   * @param options Options for the store.
   * @returns The store instance.
   *
   * @see https://maf.gilbertz.me/docs/build/store
   * @see https://maf.gilbertz.me/docs/build/type-generation
   */
  public store<K extends StoreKeys>(
    name: K,
    options?: StoreOptions<StoreSelect<K>>
  ): Store<StoreSelect<K>> {
    return super.untypedStore(name, options);
  }

  /**
   * Invoke an RPC method by name. The parameters and return type are derived
   * from the generated RPC definitions and should be serializable data.
   *
   * @param method The name of the RPC method to invoke. This should be
   * available in the generated types.
   * @param params Any number of parameters to invoke the RPC method with.
   * @returns The return value of the RPC, serialized from the server.
   *
   * @see https://maf.gilbertz.me/docs/build/rpc
   * @see https://maf.gilbertz.me/docs/build/type-generation
   */
  public rpc<K extends RpcKeys>(method: K, ...params: RpcParams<K>) {
    return this.untypedRpc<TypedRpcs[K]["result"]>(method, ...params);
  }
}
