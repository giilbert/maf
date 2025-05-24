from websockets import connect
from store import MafStore
from packet import *


class MafClient:
    def __init__(self, *, url: str, app: str):
        self.url = url
        self.app = app
        self.stores = {}

    def store(self, name: str) -> MafStore:
        if name not in self.stores:
            self.stores[name] = MafStore(self, name)
        return self.stores[name]

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
            print(f"Received from server: {packet}")

            if packet.type == "StoreUpdate":
                data = packet.store_update()
                store = self.stores.get(data.store)
                if store:
                    store.update(data.data)
                else:
                    self.stores[data.store] = MafStore(self, data.store, init=data.data)

        await self.ws.close()
        return False
