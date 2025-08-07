# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A Telegram prediction market bot where users can create and bet on yes/no predictions. Uses virtual currency (10,000 initial balance per user) and Claude AI for bet resolution.

## Development Commands

### Running the Bot
```bash
# Set required environment variables
export TELOXIDE_TOKEN="your_telegram_bot_token"
export CLAUDE_API_KEY="your_claude_api_key"

# Run with logging
RUST_LOG=info cargo run

# Run without detailed logs
cargo run
```

### Core Commands
```bash
cargo build           # Build the project
cargo check           # Fast syntax/type checking without building
cargo test            # Run tests
cargo fmt             # Format code
cargo clippy          # Lint code
```

### Database Management
```bash
rm bot.db            # Reset database (required after schema changes)
```

## Architecture

### Module Structure
- `src/main.rs` - Bot command handlers and Telegram interaction logic
- `src/db.rs` - SQLite database layer with async operations via SQLx
- `src/claude.rs` - Claude AI integration for bet resolution

### Key Components

**Command System**: Uses Teloxide's `BotCommands` derive macro for command parsing:
- `/init` - One-time balance initialization per user
- `/new <description>` - Create bet (returns bet_id)
- `/bet <bet_id> <yes/no> <amount>` - Place wager on existing bet
- `/list` - Show all bets with IDs and status
- `/solve <bet_id>` - Resolve bet (must reply to a message as proof)
- `/leaderboard` - Top 10 users by balance
- `/reset` - Admin-only database reset

**Database Schema**:
- `users` - user_id (PK), username, balance, created_at
- `bets` - bet_id (PK), creator_id (FK), description, created_at, status
- `wagers` - wager_id (PK), bet_id (FK), user_id (FK), amount, side (bool), created_at
- `solutions` - solution_id (PK), bet_id (FK), solver_id (FK), message_id, created_at
- `user_init_status` - user_id (PK), initialized, initialized_at

**Bet Resolution Flow**:
1. User replies to a message with `/solve <bet_id>`
2. Bot extracts message author and content
3. Sends to Claude API with strict JSON response format
4. Claude evaluates if message satisfies bet conditions (defaults to NO when uncertain)
5. Bot updates bet status and notifies users

**Claude Integration**:
- Model: `claude-sonnet-4-20250514`
- Strict evaluation: Defaults to rejecting solutions unless clearly valid
- Considers message author crucial for person-specific bets
- Returns JSON with `resolved` (bool) and `reasoning` (string)

## Key Patterns

- All database operations are async using SQLx
- Arc<Database> shared across handlers for thread safety
- Command handlers return `HandlerResult` for unified error handling
- Extensive logging with `log::info!` for debugging
- User balance checks before any betting operation
- Admin verification for privileged commands in group chats