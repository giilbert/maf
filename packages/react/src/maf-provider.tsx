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
    mafClient.connect(connectOptions);

    return () => {
      if (mafClient.ws.readyState === WebSocket.OPEN) {
        mafClient.ws.close();
      } else if (mafClient.ws.readyState === WebSocket.CONNECTING) {
        mafClient.ws.onopen = () => {
          mafClient.ws.close();
        };
      }
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
