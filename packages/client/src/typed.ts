import { MafUntypedBaseClient, type MafClientOptions } from "./client";
import type { Store, StoreOptions } from "./store";

export interface StoreDefinition<S> {
  name: string;
  select: S;
}

interface DefaultMafTypes {
  generated: {
    stores: Record<string, StoreDefinition<unknown>>;
  };
}

export interface MafTypes extends DefaultMafTypes {}

type TypedStores = MafTypes["generated"]["stores"];
type StoreKeys = keyof TypedStores;
type StoreSelect<K extends StoreKeys> = TypedStores[K]["select"];

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

  public rpc<T>(method: string, ...params: unknown[]) {
    return this.untypedRpc<T>(method, ...params);
  }
}
