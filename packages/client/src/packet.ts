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
    };
