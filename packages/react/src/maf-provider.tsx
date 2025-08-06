import { MafClient } from "@usemaf/client";
import { ConnectOptions, MafServerOptions } from "@usemaf/client/src/client";
import React, { createContext, useEffect, useState } from "react";

export interface MafProviderProps {
  server: MafServerOptions;
  connectOptions?: ConnectOptions;
}

export interface MafContextType {
  client: MafClient;
}

export const MafContext = createContext<MafContextType | null>(null);

export const useMaf = () => {
  const context = React.useContext(MafContext);
  if (!context) throw new Error("useMaf must be used within a MafProvider");
  return context.client;
};

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
