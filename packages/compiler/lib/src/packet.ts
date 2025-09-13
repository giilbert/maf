type RxPacket = {
  type: "TypedRpcCall";
  data: {
    method: string;
    id: bigint;
    params: unknown;
  };
};
