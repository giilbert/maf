import { useContext } from "react";
import { MafContext } from "./maf-provider";
import { useMutation, UseMutationResult } from "@tanstack/react-query";

export type UseRpc<TData> = UseMutationResult<TData, Error, unknown, unknown>;

export function useRpc<TData>(method: string): UseRpc<TData> {
  const contextData = useContext(MafContext);

  if (!contextData) {
    throw new Error(
      "MafContext is not available. Ensure you are within a MafProvider."
    );
  }

  const { client } = contextData;

  const mutation = useMutation({
    mutationFn: (...args: unknown[]) => client.rpc<TData>(method, ...args),
  });

  return mutation;
}
