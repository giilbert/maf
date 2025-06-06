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

        async def update_counter():
            while True:
                await client.rpc("increment_counter", 1)
                await asyncio.sleep(1)

        asyncio.create_task(counter_store())
        asyncio.create_task(update_counter())

        pass


if __name__ == "__main__":
    asyncio.run(main())
