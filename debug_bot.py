import asyncio
import os
import logging
from telegram import Bot

# Set up basic logging to see what's happening under the hood
logging.basicConfig(format="%(asctime)s - %(name)s - %(levelname)s - %(message)s", level=logging.DEBUG)


async def main():
    # Try to read token from env var, or parse .env file manually
    token = os.getenv("BOT_TOKEN")
    if not token and os.path.exists(".env"):
        print("Reading .env file manually...")
        with open(".env", "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line.startswith("BOT_TOKEN="):
                    token = line.split("=", 1)[1].strip()
                    break

    if not token:
        print("Error: No BOT_TOKEN found in environment or .env file.")
        return

    print(f"Testing connection with token: {token[:6]}***")

    try:
        # Initialize Bot
        bot = Bot(token)
        print("Bot instance created. Calling get_me()...")

        # Call get_me
        me = await bot.get_me()
        print(f"✅ SUCCESS! Bot connected.")
        print(f"ID: {me.id}")
        print(f"Username: @{me.username}")
        print(f"Full Name: {me.first_name}")

    except Exception as e:
        print(f"❌ ERROR: {e}")
        import traceback

        traceback.print_exc()


if __name__ == "__main__":
    asyncio.run(main())
