from websockets import connect
from store import CobbleStore
from packet import *
import asyncio


class CobbleClient:
    def __init__(self, *, url: str, app: str):
        self.url = url
        self.app = app
        self.stores = {}

        self.rpc_id = 0
        self.rpc_calls = {}

    def store(self, name: str) -> CobbleStore:
        if name not in self.stores:
            self.stores[name] = CobbleStore(self, name)
        return self.stores[name]

    async def rpc(self, method: str, *params):
        params = params if len(params) > 1 else params[0] if params else None
        self.rpc_id += 1
        packet = TxTypedRpcCallPacket(
            data=TxTypedRpcCallData(id=self.rpc_id, method=method, params=params),
            type="TypedRpcCall",
        )
        queue = asyncio.Queue()
        self.rpc_calls[self.rpc_id] = queue
        await self.ws.send(packet.to_json())
        return await queue.get()

    async def __aenter__(self):
        self.ws = await connect(
            f"{self.url.replace('http', 'ws')}/@/{self.app}/connect"
        )

        # TODO: handle custom auth
        await self.ws.send(
            TxHandshakePacket(data=TxHandshakeData(auth="hello")).to_json()
        )

        return self

    async def __aexit__(self, *_args):
        async for message in self.ws:
            packet = RxPacket(message)
            # print(f"Received from server: {packet}")

            if packet.type == "StoreUpdate":
                data = packet.store_update()
                store = self.stores.get(data.store)
                if store:
                    store.update(data.data)
                else:
                    self.stores[data.store] = CobbleStore(self, data.store, init=data.data)
            elif packet.type == "ManyStoreUpdate":
                for data in packet.many_store_update():
                    if data.store not in self.stores:
                        self.stores[data.store] = CobbleStore(
                            self, data.store, init=data.data
                        )
                    else:
                        self.stores[data.store].update(data.data)
            elif packet.type == "TypedRpcResponse":
                data = packet.typed_rpc_response()
                if data.id in self.rpc_calls:
                    queue = self.rpc_calls.pop(data.id)
                    queue.put_nowait(data.result)

        await self.ws.close()
        return False
