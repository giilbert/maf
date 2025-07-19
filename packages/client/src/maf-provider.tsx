import { MafClient } from "./client";
import React, { createContext } from "react";

export interface MafProviderProps {
  url: string;
  app: string;
}

export interface MafContextType {
  client: MafClient;
}

export const MafContext = createContext<MafContextType | null>(null);

const MafProvider: React.FC<React.PropsWithChildren<MafProviderProps>> = ({
  url,
  app,
  children,
}) => {
  const client = new MafClient({ url, app });
  client.connect();

  const contextValue: MafContextType = { client };

  return (
    <MafContext.Provider value={contextValue}>{children}</MafContext.Provider>
  );
};

export default MafProvider;
