import { useEffect, useState } from "react";
import { useMaf } from "../maf-provider";
import { SessionInfo } from "@usemaf/client";
import { MafStatus } from "./use-store";

export type UseSession =
  | { status: MafStatus.LOADING; data: null }
  | { status: MafStatus.READY; data: SessionInfo };

export function useSession(): UseSession {
  const [status, setStatus] = useState<MafStatus>(MafStatus.LOADING);
  const client = useMaf();

  useEffect(() => {
    const unsubscribe = client.on("ready", () => {
      setStatus(MafStatus.READY);
    });

    return () => {
      unsubscribe();
    };
  }, [client]);

  if (status === MafStatus.LOADING) {
    return {
      status: MafStatus.LOADING,
      data: null,
    };
  }

  return {
    status: MafStatus.READY,
    data: client.sessionInfo,
  };
}
