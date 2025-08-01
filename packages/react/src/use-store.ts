import { useEffect, useState, useContext } from "react";
import { MafContext } from "./maf-provider";

export enum MafStatus {
  LOADING = "loading",
  READY = "ready",
}

type UseStoreDiscUnion<TData, TFallback> =
  | {
      status: MafStatus.LOADING;
      data: TFallback;
    }
  | {
      status: MafStatus.READY;
      data: TData;
    };

export function useStore<TData>(
  storeName: string
): UseStoreDiscUnion<TData, never>;

export function useStore<TData, TFallback extends TData>(
  storeName: string,
  fallback: TFallback
): UseStoreDiscUnion<TData, TFallback>;

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
  const [status, setStatus] = useState<MafStatus>(MafStatus.LOADING);
  const [data, setData] = useState<TData | TFallback>(
    fallback as TFallback extends undefined ? never : TFallback
  );

  const contextData = useContext(MafContext);

  useEffect(() => {
    if (contextData !== null) {
      const { client } = contextData;
      const store = client.store<TData>(storeName);
      store.init.then(() => {
        setData(store.data);
        setStatus(MafStatus.READY);
        store.on("change", () => setData(store.data));
      });
    }
  }, [contextData]);

  if (status === MafStatus.LOADING) {
    return {
      status: MafStatus.LOADING,
      data: fallback as TFallback extends undefined ? never : TFallback,
    };
  }

  return {
    status: MafStatus.READY,
    data: data as TData,
  };
}
