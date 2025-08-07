# Telegram Betting Bot

A Telegram bot for managing bets in group chats with a virtual balance system.

## Features

- **Balance System**: Each user gets 10,000 initial balance (one-time)
- **Prediction Markets**: Create yes/no prediction markets
- **Wagering**: Bet on either side of a prediction
- **AI Resolution**: Claude AI evaluates if solutions resolve bets
- **Leaderboard**: Track top users by balance

## Setup

1. Create a new bot on Telegram using @BotFather
2. Get your bot token
3. Get a Claude API key from https://console.anthropic.com/
4. Set the environment variables:
   ```bash
   export TELOXIDE_TOKEN="your_bot_token_here"
   export CLAUDE_API_KEY="your_claude_api_key_here"
   ```

## Running the Bot

```bash
cargo run
```

The bot will create a SQLite database file (`bot.db`) to store user balances and bets.

## Commands

- `/init` - Get your initial 10,000 balance (one-time per user)
- `/new <title> <description>` - Create a new bet/prediction market
- `/bet <title> <yes/no> <amount>` - Place a wager on an existing bet
- `/solve <bet_id>` - Mark a bet as solved (reply to a message, uses Claude AI to verify)
- `/leaderboard` - Show top 10 users by balance
- `/reset` - Admin-only command to reset the entire database
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