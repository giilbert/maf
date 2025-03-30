type TxPacket = {
  type: "ChannelSend";
  data: {
    channel: string;
    data: unknown;
  };
};
