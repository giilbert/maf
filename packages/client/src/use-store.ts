import { useEffect, useState, useContext } from "react";
import { MafContext } from "./maf-provider";

export enum StoreStatus {
  LOADING,
  READY,
}

type UseStoreDiscUnion<TData, TFallback> =
  | {
      status: StoreStatus.LOADING;
      data: TFallback;
    }
  | {
      status: StoreStatus.READY;
      data: TData;
    };

function useStore<TData>(storeName: string): UseStoreDiscUnion<TData, never>;

function useStore<TData, TFallback extends TData>(
  storeName: string,
  fallback: TFallback
): UseStoreDiscUnion<TData, TFallback>;

function useStore<TData, TFallback>(
  storeName: string,
  fallback?: TFallback
): UseStoreDiscUnion<
  TData | undefined,
  TFallback extends undefined ? never : TFallback
> {
  const [status, setStatus] = useState<StoreStatus>(StoreStatus.LOADING);
  const [data, setData] = useState<TData | TFallback>(
    fallback as TFallback extends undefined ? never : TFallback
  );

  const contextData = useContext(MafContext);

  useEffect(() => {
    if (contextData !== null) {
      const { client } = contextData;
      const store = client.store<TData>(storeName);
      if (store.hasInit) {
        store.on("change", () => {
          setData(store.data);
          setStatus(StoreStatus.READY);
        });
      }
    }
  }, [contextData]);

  if (status === StoreStatus.LOADING) {
    return {
      status: StoreStatus.LOADING,
      data: fallback as TFallback extends undefined ? never : TFallback,
    };
  }

  return {
    status: StoreStatus.READY,
    data: data as TData,
  };
}
