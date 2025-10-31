import { CobbleClient } from "@usecobble/client";
import {
  ConnectOptions,
  CobbleServerOptions,
} from "@usecobble/client/src/client";
import React, { createContext, useEffect, useState } from "react";

export interface CobbleProviderProps {
  server: CobbleServerOptions;
  connectOptions?: ConnectOptions;
}

export interface CobbleContextType {
  client: CobbleClient;
}

export const CobbleContext = createContext<CobbleContextType | null>(null);

export const useCobble = () => {
  const context = React.useContext(CobbleContext);
  if (!context)
    throw new Error("useCobble must be used within a CobbleProvider");
  return context.client;
};

export const CobbleProvider: React.FC<
  React.PropsWithChildren<CobbleProviderProps>
> = ({ server, connectOptions = { type: "default" }, children }) => {
  const [cobbleClient] = useState<CobbleClient>(() => {
    return new CobbleClient({ server });
  });

  useEffect(() => {
    // Connect will throw an error if the connection fails or cancels, which is
    // expected when React strict mode is enabled.
    // TODO: Pass other errors through?
    cobbleClient.connect(connectOptions).catch(() => {});

    return () => {
      cobbleClient.disconnect();
    };
  }, []);

  return (
    <CobbleContext.Provider value={{ client: cobbleClient }}>
      {children}
    </CobbleContext.Provider>
  );
};
