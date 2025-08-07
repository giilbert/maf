import { MafUntypedBaseClient, type MafClientOptions } from "./client";
import type { Store, StoreOptions } from "./store";

export interface StoreDefinition<S> {
  name: string;
  select: S;
}

export interface RpcDefinition<P, R extends unknown | unknown[]> {
  name: string;
  params: P;
  result: R;
}

interface DefaultMafTypes {
  generated: {
    stores: Record<string, StoreDefinition<unknown>>;
    rpcs: Record<string, RpcDefinition<unknown, unknown>>;
  };
}

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

  public store<K extends StoreKeys>(
    name: K,
    options?: StoreOptions<StoreSelect<K>>
  ): Store<StoreSelect<K>> {
    return super.untypedStore(name, options);
  }

  public rpc<K extends RpcKeys>(method: K, ...params: RpcParams<K>) {
    return this.untypedRpc<TypedRpcs[K]["result"]>(method, ...params);
  }
}
