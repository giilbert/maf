import { useContext } from "react";
import { MafContext } from "./maf-provider";
import { SessionInfo } from "@usemaf/client/src/client";

export function useSession(): SessionInfo {
  const contextData = useContext(MafContext);

  if (!contextData) {
    throw new Error(
      "MafContext is not available. Ensure you are within a MafProvider."
    );
  }

  return contextData.client.sessionInfo;
}
