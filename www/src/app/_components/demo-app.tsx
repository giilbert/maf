"use client";

import { CircleIcon } from "lucide-react";
import { MafClient } from "@usemaf/client";
import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/cn";

interface StoreData {
  tiles: boolean[];
  people: number;
}

export const DemoApp: React.FC = () => {
  const [tiles, setTiles] = useState<boolean[]>(Array(64).fill(false));
  const [people, setPeople] = useState<number | null>(null);
  const [connectionStatus, setConnectionStatus] = useState<
    | { type: "connecting" }
    | { type: "connected" }
    | {
        type: "disconnected";
        reason?: string;
      }
  >({ type: "connecting" });
  const mafRef = useRef<MafClient>(null);

  useEffect(() => {
    const maf = new MafClient({
      server:
        process.env.NODE_ENV === "development"
          ? "dev"
          : {
              type: "platform",
              url: "https://maf-server.fly.dev",
              app: "gilbert/maf-demo",
            },
    });

    mafRef.current = maf;

    const store = maf.store<StoreData>("LightsOut");
    store.init.then(() => {
      setTiles(store.data.tiles);
      setPeople(store.data.people);
    });
    store.on("change", (data) => {
      setTiles(data.tiles);
      setPeople(store.data.people);
    });

    maf
      .connect()
      .then(() => setConnectionStatus({ type: "connected" }))
      .catch(() =>
        setConnectionStatus({
          type: "disconnected",
          reason: "Failed to connect",
        })
      );

    maf.once("close", () => {
      console.log("Connection closed...");
      console.log("Reconnecting...");

      setConnectionStatus({
        type: "disconnected",
        reason: "Trying to reconnect...",
      });

      const interval = setInterval(() => {
        maf
          .connect()
          .then(() => {
            setConnectionStatus({ type: "connected" });
            clearInterval(interval);
          })
          .catch(() =>
            setConnectionStatus({
              type: "disconnected",
              reason: "Failed to connect",
            })
          );
      }, 2000);

      return false;
    });

    return () => {
      if (maf.ws.readyState === WebSocket.OPEN) {
        maf.ws.close();
        mafRef.current = null;
      } else if (maf.ws.readyState === WebSocket.CONNECTING) {
        maf.ws.onopen = () => {
          maf.ws.close();
          mafRef.current = null;
        };
      }
    };
  }, []);

  return (
    <div className="w-full h-full flex flex-col">
      <div className="text-sm z-10 p-4">
        <div className="flex items-center gap-2 font-mono w-full text-sm">
          {connectionStatus.type === "connected" ? (
            <>
              <CircleIcon
                size={16}
                className="-mt-1 text-red-500"
                fill="currentColor"
              />
              <p className="text-red-500">LIVE DEMO</p>
              {people ? (
                <p className="text-muted-foreground">
                  ({people} {people === 1 ? "person" : "people"} online)
                </p>
              ) : (
                <p>...</p>
              )}
            </>
          ) : connectionStatus.type === "disconnected" ? (
            <p>
              {connectionStatus.reason && (
                <span className="text-red-500">{connectionStatus.reason}</span>
              )}
            </p>
          ) : (
            <p className="text-muted-foreground">Connecting...</p>
          )}
          <p className="ml-auto">Lights Out - Synchronized Between Everyone</p>
        </div>
      </div>
      <div className="flex items-center justify-center w-full h-full p-4 relative">
        <div className="grid grid-cols-8 grid-rows-8 gap-1 aspect-square">
          {tiles.map((isOn, index) => (
            <div
              key={index}
              onClick={() => {
                const maf = mafRef.current;
                if (!maf) return;

                maf.rpc("toggle_tile", index);
              }}
              className={cn(
                "transform scale-100 hover:border cursor-pointer hover:scale-105 flex items-center justify-center max-w-full max-h-full w-16 h-16 transition-all",
                isOn
                  ? "bg-neutral-400 hover:bg-neutral-500"
                  : "bg-neutral-900 hover:bg-neutral-800"
              )}
            />
          ))}
        </div>
      </div>
    </div>
  );
};
