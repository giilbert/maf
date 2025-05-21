import asyncio
from websockets import connect
from client import MafClient
import logging

logging.basicConfig(
    format="%(asctime)s %(message)s",
    level=logging.INFO,
)


async def main():
    async with MafClient(
        url="http://localhost:3000",
        app="gilbert/test-2",
    ) as client:

        async def counter_store():
            store = client.store("example_basic::CounterStore")
            async for data in store.changed():
                print(f"store changed: {data}")

        asyncio.create_task(counter_store())

        pass

    # async with connect("ws://localhost:3000/@/hello/hello/connect") as ws:
    #     await ws.send(TxHandshakePacket(
    #         data = TxHandshakeData(
    #             auth="hello"
    #         ),
    #     ).to_json())

    #     response = RxPacket(await ws.recv())
    #     print(f"Received from server: {response}")

    #     async for message in ws:
    #         response = RxPacket(message)

    #         if response.type == "StoreUpdate":
    #             data = response.store_update()
    #             print(f"StoreUpdate data: {data}")

    #         print(f"Received from server: {response}")


if __name__ == "__main__":
    asyncio.run(main())
