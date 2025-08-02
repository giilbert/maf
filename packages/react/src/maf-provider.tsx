import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MafClient } from "@usemaf/client";
import { ConnectOptions, MafServerOptions } from "@usemaf/client/src/client";
import React, { createContext, useEffect, useRef, useState } from "react";

export interface MafProviderProps {
  server: MafServerOptions;
  connectOptions?: ConnectOptions;
}

export interface MafContextType {
  client: MafClient;
}

export const MafContext = createContext<MafContextType | null>(null);
const queryClient = new QueryClient();

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
    <MafContext.Provider
      value={
        {
          client: mafClient,
        } as MafContextType
      }>
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    </MafContext.Provider>
  );
};
