type TxPacket = {
  type: "ChannelSend";
  data: {
    channel: string;
    data: unknown;
  };
};

type RxPacket = {
  type: "ChannelSend";
  data: {
    channel: string;
    data: unknown;
  };
};
