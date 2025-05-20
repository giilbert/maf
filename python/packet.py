import json
import dataclasses
from dataclasses import dataclass
from typing import TypeVar, Literal, Any

T = TypeVar("T")

@dataclass
class RxHandshakeData:
    id: str

@dataclass
class RxChannelSendData:
    channel: str
    data: Any

@dataclass
class RxStoreUpdateData:
    store: str
    data: Any

@dataclass
class RxTypedRpcResponseData:
    id: str
    result: Any

@dataclass
class RxPacket:
    data: T
    type: str

    def __init__(self, data: str):
        raw = json.loads(data)
        self.data = raw.get("data")
        self.type = raw.get("type")
    
    def handshake(self) -> RxHandshakeData:
        assert self.type == "Handshake"
        return RxHandshakeData(**self.data)
    
    def channel_send(self) -> RxChannelSendData:
        assert self.type == "ChannelSend"
        return RxChannelSendData(**self.data)

    def store_update(self) -> RxStoreUpdateData:
        assert self.type == "StoreUpdate"
        return RxStoreUpdateData(**self.data)

    def many_store_update(self) -> list[RxStoreUpdateData]:
        assert self.type == "ManyStoreUpdate"
        return [RxStoreUpdateData(**item) for item in self.data]

    def typed_rpc_response(self) -> RxTypedRpcResponseData:
        assert self.type == "TypedRpcResponse"
        return RxTypedRpcResponseData(**self.data)

class DataclassJSONEncoder(json.JSONEncoder):
    def default(self, o):
        if dataclasses.is_dataclass(o):
            return dataclasses.asdict(o)
        return super().default(o)

@dataclass
class TxPacket:
    data: T
    type: str

    def __init__(self, data: T, type: str):
        self.data = data
        self.type = type

    def to_json(self) -> str:
        return json.dumps({"data": self.data, "type": self.type}, cls=DataclassJSONEncoder)
    
@dataclass
class TxHandshakeData:
    auth: T

@dataclass
class TxHandshakePacket(TxPacket):
    data: TxHandshakeData
    type: Literal["Handshake"] = "Handshake"
