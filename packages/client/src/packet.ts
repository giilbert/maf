type TxPacket =
  | {
      type: "ChannelSend";
      data: {
        channel: string;
        data: unknown;
      };
    }
  | {
      type: "TypedRpcCall";
      data: {
        method: string;
        id: number;
        params: unknown;
      };
    };

type RxPacket =
  | {
      type: "ChannelSend";
      data: {
        channel: string;
        data: unknown;
      };
    }
  | {
      type: "TypedRpcResponse";
      data: {
        id: number;
        result: unknown;
      };
    }
  | {
      type: "ManyStoreUpdate";
      data: {
        store: string;
        data: unknown;
      }[];
    }
  | {
      type: "StoreUpdate";
      data: {
        store: string;
        data: unknown;
      };
    };
