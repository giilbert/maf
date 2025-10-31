import { useEffect, useState } from "react";
import { useCobble } from "../cobble-provider";
import { SessionInfo } from "@usecobble/client/src/client";
import { CobbleStatus } from "./use-store";

export type UseSession =
  | { status: CobbleStatus.LOADING; data: null }
  | { status: CobbleStatus.READY; data: SessionInfo };

export function useSession(): UseSession {
  const [status, setStatus] = useState<CobbleStatus>(CobbleStatus.LOADING);
  const client = useCobble();

  useEffect(() => {
    const unsubscribe = client.on("ready", () => {
      setStatus(CobbleStatus.READY);
    });

    return () => {
      unsubscribe();
    };
  }, [client]);

  if (status === CobbleStatus.LOADING) {
    return {
      status: CobbleStatus.LOADING,
      data: null,
    };
  }

  return {
    status: CobbleStatus.READY,
    data: client.sessionInfo,
  };
}
