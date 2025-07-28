import { MafClient } from "@usemaf/client";
import { ConnectOptions } from "@usemaf/client/src/client";
import React, { createContext, useEffect, useRef, useState } from "react";

export interface MafProviderProps {
  url: string;
  app: string;
  connectOptions?: ConnectOptions;
}

export interface MafContextType {
  client: MafClient;
}

export const MafContext = createContext<MafContextType | null>(null);

export const MafProvider: React.FC<
  React.PropsWithChildren<MafProviderProps>
> = ({ url, app, connectOptions = { type: "default" }, children }) => {
  const [mafClient] = useState<MafClient>(() => {
    return new MafClient({ url, app });
  });

  useEffect(() => {
    // Steam roll errors because react strict mode is dumb
    mafClient.connect(connectOptions).catch(() => {});

    return () => {
      mafClient.disconnect();
    };
  }, [url, app]);

  return (
    <MafContext.Provider
      value={
        {
          client: mafClient,
        } as MafContextType
      }>
      {children}
    </MafContext.Provider>
  );
};
