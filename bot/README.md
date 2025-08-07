# Telegram Betting Bot

A Telegram bot for managing bets in group chats with a virtual balance system.

## Features

- **Balance System**: Each user gets 10,000 initial balance
- **Bet Creation**: Users can create bets with descriptions
- **Solution Recording**: Users can mark bets as solved by replying to messages

## Setup

1. Create a new bot on Telegram using @BotFather
2. Get your bot token
3. Set the environment variable:
   ```bash
   export TELOXIDE_TOKEN="your_bot_token_here"
   ```

## Running the Bot

```bash
cargo run
```

The bot will create a SQLite database file (`bot.db`) to store user balances and bets.

## Commands

- `/init` - Admin-only command to initialize 10,000 balance for all users in the group
- `/bet <description>` - Create a new bet with a description
- `/solve` - Mark a bet as solved (must be used as a reply to a message)
- `/help` - Show available commands

## Database Schema

The bot uses SQLite with three tables:

1. **users**: Stores user information and balances
2. **bets**: Stores created bets with descriptions
3. **solutions**: Stores solutions linked to bets and messages

## Development

```bash
# Build the project
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Notes

- The `/init` command can only be used by group admins
- Users are automatically given 10,000 balance when they first interact with the bot
- Currently, the `/solve` command records solutions but doesn't link them to specific bets (placeholder functionality)