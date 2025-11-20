import asyncio
import os
import logging
from telegram.ext import ApplicationBuilder

logging.basicConfig(format="%(asctime)s - %(name)s - %(levelname)s - %(message)s", level=logging.DEBUG)


async def main():
    token = None
    proxy = None

    if os.path.exists(".env"):
        with open(".env", "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line.startswith("BOT_TOKEN="):
                    token = line.split("=", 1)[1].strip()
                elif line.startswith("PROXY_URL="):
                    proxy = line.split("=", 1)[1].strip()

    if not token:
        print("❌ No BOT_TOKEN found.")
        return

    print(f"Testing with Proxy: {proxy}")

    try:
        builder = ApplicationBuilder().token(token)
        if proxy:
            builder.proxy(proxy)

        app = builder.build()

        # Application must be initialized/started to use the bot's network in some cases,
        # but get_me() usually works if initialized.
        async with app:  # Initialize app context
            me = await app.bot.get_me()
            print(f"✅ SUCCESS! Connected as {me.username}")

    except Exception as e:
        print(f"❌ ERROR: {e}")
        import traceback

        traceback.print_exc()


if __name__ == "__main__":
    asyncio.run(main())
