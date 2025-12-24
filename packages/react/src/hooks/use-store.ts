import { useEffect, useState } from "react";
import { useMafClient } from "../maf-provider";
import type { StoreKeys, StoreSelect } from "@usemaf/client";

type StoreStatus = "loading" | "ready";

type UseStoreDiscUnion<TData, TFallback> =
  | {
      status: "loading";
      data: TFallback;
    }
  | {
      status: "ready";
      data: TData;
    };

/**
 * Subscribes to a *typed* store and reactively provides its data when ready.
 *
 * *The data will be be `undefined` until the store is ready.*
 *
 * @param storeName The name of the store to subscribe to. This must be
 * a valid store name defined in the generated types.
 */
export function useStore<TName extends StoreKeys>(
  storeName: TName
): UseStoreDiscUnion<StoreSelect<TName>, never>;

/**
 * Subscribes to a *typed* store and reactively provides its data when ready.
 *
 * *The data will be be a specified fallback value until the store is ready.*
 *
 * @param storeName The name of the store to subscribe to. This must be
 * a valid store name defined in the generated types.
 * @param fallback The fallback data to use while the store is loading.
 */
export function useStore<TName extends StoreKeys>(
  storeName: string,
  fallback?: StoreSelect<TName>
): UseStoreDiscUnion<StoreSelect<TName>, StoreSelect<TName>>;

/**
 * Subscribes to a *typed* store and reactively provides its data when ready.
 *
 * *The data will be be a specified fallback value until the store is ready.*
 *
 * @param storeName The name of the store to subscribe to. This must be
 * a valid store name defined in the generated types.
 * @param fallback The fallback data to use while the store is loading. *The
 * fallback data is different from the store data type.*
 */
export function useStore<TName extends StoreKeys, TFallback>(
  storeName: TName,
  fallback?: TFallback
): UseStoreDiscUnion<StoreSelect<TName>, TFallback>;

/**
 * Subscribes to a store and reactively provides its data when ready. Use the
 * type parameter `TData` to specify the expected data shape.
 *
 * *The data will be be `undefined` until the store is ready.*
 *
 * @param storeName The name of the store to subscribe to.
 */
export function useStore<TData>(
  storeName: string
): UseStoreDiscUnion<TData, never>;

/**
 * Subscribes to a store and reactively provides its data when ready. Use the
 * type parameter `TData` to specify the expected data shape.
 *
 * *The data will be be a specified fallback value until the store is ready.*
 *
 * @param storeName The name of the store to subscribe to.
 * @param fallback The fallback data to use while the store is loading.
 */
export function useStore<TData, TFallback extends TData>(
  storeName: string,
  fallback: TFallback
): UseStoreDiscUnion<TData, TFallback>;

/**
 * Subscribes to a store and reactively provides its data when ready. Use the
 * type parameter `TData` to specify the expected data shape.
 *
 * *The data will be be a specified fallback value until the store is ready.*
 *
 * @param storeName The name of the store to subscribe to.
 * @param fallback The fallback data to use while the store is loading.
 */
export function useStore<TData>(
  storeName: string,
  fallback?: TData
): UseStoreDiscUnion<TData, TData>;

export function useStore<TData, TFallback>(
  storeName: string,
  fallback?: TFallback
): UseStoreDiscUnion<
  TData | undefined,
  TFallback extends undefined ? never : TFallback
> {
  const [status, setStatus] = useState<StoreStatus>("loading");
  const [data, setData] = useState<TData | TFallback>(
    fallback as TFallback extends undefined ? never : TFallback
  );
  const client = useMafClient();

  useEffect(() => {
    const store = client.store<TData>(storeName);

    store.init.then(() => {
      setData(store.data);
      setStatus("ready");
      store.on("change", () => setData(store.data));
    });
  }, [client]);

  if (status === "loading") {
    return {
      status: "loading",
      data: fallback as TFallback extends undefined ? never : TFallback,
    };
  }

  return {
    status: "ready",
    data: data as TData,
  };
}
