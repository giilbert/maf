import { useEffect, useState } from "react";
import { useMafClient } from "../maf-provider";
import { SessionInfo } from "@usemaf/client";

type UseSessionStatus = "loading" | "ready";

/**
 * The type returned by the {@link useSession} hook. Represents session loading
 * state.
 */
export type UseSession =
  | { status: "loading"; data: null }
  | { status: "ready"; data: SessionInfo };

/**
 * Subscribes to session status and provides session information when ready.
 *
 * @example
 * function App() {
 *   const session = useSession();
 *
 *   // Use discriminated union to handle loading and ready states.
 *   if (session.status === "loading") {
 *     return <p>Loading...</p>;
 *   }
 *
 *   // session.status is "ready" here
 *   return <p>Session User ID: {session.data.id}</p>;
 * }
 *
 * @returns {UseSession} The current session status and data.
 */
export function useSession(): UseSession {
  const [status, setStatus] = useState<UseSessionStatus>("loading");
  const client = useMafClient();

  useEffect(() => {
    const unsubscribe = client.on("ready", () => {
      setStatus("ready");
    });

    return () => {
      unsubscribe();
    };
  }, [client]);

  if (status === "loading") {
    return {
      status: "loading",
      data: null,
    };
  }

  return {
    status: "ready",
    data: client.sessionInfo,
  };
}
