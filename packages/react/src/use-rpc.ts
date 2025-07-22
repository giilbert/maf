import { useContext } from "react";
import { MafContext } from "./maf-provider";

export interface UseRPC<TData> {
  mutateAsync: () => Promise<TData>;
}

export function useRPC<TData>(
  method: string,
  ...args: unknown[]
): UseRPC<TData> {
  const contextData = useContext(MafContext);

  if (!contextData) {
    throw new Error(
      "MafContext is not available. Ensure you are within a MafProvider."
    );
  }

  const { client } = contextData;

  return {
    mutateAsync: () => client.rpc<TData>(method, ...args),
  };
}
