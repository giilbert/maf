import {
  ConnectOptions,
  MafClient,
  MafServerOptions,
  MafTypes,
  TypedMafClient,
} from "@usemaf/client";
import React, { createContext, useEffect, useState } from "react";

export interface MafProviderProps {
  /**
   * What server to connect to.
   * @see MafServerOptions
   */
  server: MafServerOptions;
  /**
   * Options for connecting to the server. You will need to use this to:
   * - Include authentication credentials.
   * - Specify which room to connect to.
   */
  connectOptions?: ConnectOptions;
}

export interface MafContextType {
  client: MafClient;
}

export const MafContext = createContext<MafContextType | null>(null);

/**
 * Hook to access the `MafClient` instance from context.
 *
 * @example
 * function CounterButton() {
 *   const maf = useMafClient();
 *
 *   return (
 *     <button onClick={() => maf.rpc("increment_counter", 1)}>
 *       Increase the counter!
 *     </button>
 *   );
 * }
 *
 * @returns An instance of {@link MafClient}, provided by the nearest
 * {@link MafProvider}.
 */
export function useMafClient(): MafTypes["generated"]["__isTyped"] extends true
  ? TypedMafClient
  : MafClient {
  const context = React.useContext(MafContext);
  if (!context) throw new Error("useMaf must be used within a MafProvider");
  return context.client;
}

/**
 * Provider component that initializes and provides a {@link MafClient} instance
 * to its child components via context.
 */
export const MafProvider: React.FC<
  React.PropsWithChildren<MafProviderProps>
> = ({ server, connectOptions = { type: "default" }, children }) => {
  const [mafClient] = useState<MafClient>(() => {
    return new MafClient({ server });
  });

  useEffect(() => {
    // Connect will throw an error if the connection fails or cancels, which is
    // expected when React strict mode is enabled.
    // TODO: Pass other errors through?
    mafClient.connect(connectOptions).catch(() => {});

    return () => {
      mafClient.disconnect();
    };
  }, []);

  return (
    <MafContext.Provider value={{ client: mafClient }}>
      {children}
    </MafContext.Provider>
  );
};
