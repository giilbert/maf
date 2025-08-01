import { useContext, useEffect, useState } from "react";
import { MafContext } from "./maf-provider";
import { SessionInfo } from "@usemaf/client/src/client";
import { MafStatus } from "./use-store";

export type UseSession =
  | { status: MafStatus.LOADING; data: null }
  | { status: MafStatus.READY; data: SessionInfo };

export function useSession(): UseSession {
  const [status, setStatus] = useState<MafStatus>(MafStatus.LOADING);
  const contextData = useContext(MafContext);
  if (!contextData)
    throw new Error("useSession used outside of a <M>afProvider");

  useEffect(() => {
    const { client } = contextData;

    const handleReady = () => {
      setStatus(MafStatus.READY);
    };

    client.on("ready", handleReady);

    return () => {
      client.off("ready", handleReady);
    };
  }, [contextData]);

  if (status === MafStatus.LOADING) {
    return {
      status: MafStatus.LOADING,
      data: null,
    };
  }

  return {
    status: MafStatus.READY,
    data: contextData.client.sessionInfo,
  };
}
