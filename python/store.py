import asyncio
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from client import CobbleClient


class CobbleStore:
    def __init__(self, client: "CobbleClient", name: str, init=None):
        self.client = client
        self.name = name

        self.__data = init
        self.change_channel = asyncio.Queue()

    def update(self, data):
        self.__data = data
        self.change_channel.put_nowait(data)

    def get(self):
        return self.__data

    def changed(self):
        return CobbleStoreChanged(self)


class CobbleStoreChanged:
    def __init__(self, store: CobbleStore):
        self.store = store

    def __aiter__(self):
        return self

    async def __anext__(self):
        return await self.store.change_channel.get()
