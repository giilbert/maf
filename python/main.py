import asyncio
from websockets import connect
import logging

logging.basicConfig(
    format="%(asctime)s %(message)s",
    level=logging.INFO,
)

from packet import *

async def main():
    async with connect("ws://localhost:3000/@/hello/hello/connect") as ws:
        await ws.send(TxHandshakePacket(
            data = TxHandshakeData(
                auth="hello"
            ),
        ).to_json())

        response = RxPacket(await ws.recv())
        print(f"Received from server: {response}")

        async for message in ws:
            response = RxPacket(message)

            if response.type == "StoreUpdate":
                data = response.store_update()
                print(f"StoreUpdate data: {data}")

            print(f"Received from server: {response}")

if __name__ == "__main__":
    asyncio.run(main())