# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a parimutuel betting market system built with Rust, consisting of:
- **bot/**: Telegram bot for user interaction
- **contracts/**: Zero-knowledge smart contracts (RISC0/RISC-V)
- **server/**: REST API server and blockchain node interface

The system uses Hyle blockchain SDK (v0.13.0) and implements a decentralized betting platform where users can create markets, place bets, and claim winnings through Telegram.

## Essential Commands

### Development
```bash
# Bot development (requires env vars)
cd bot
export TELOXIDE_TOKEN="your_telegram_token"
export CLAUDE_API_KEY="your_claude_api_key"
cargo run

# Server development
cd server
cargo run -- --config ../config.toml

# Contract compilation (reproducible build)
cd contracts
cargo build --features build,contract1

# Contract compilation (local/fast build)
cd contracts
cargo build --features nonreproducible,contract1
```

### Code Quality
```bash
# Format all code
cargo fmt --all

# Run linter
cargo clippy --all-targets --all-features

# Run tests (limited test coverage currently)
cargo test --workspace
```

## Architecture

### Workspace Structure
The project uses Cargo workspaces with centralized dependency management in root `Cargo.toml`. Key workspace members:
- `contracts/contract1` - Core smart contract logic
- `bot` - Telegram interface
- `server` - REST API and blockchain integration

### Contract System
Contracts are compiled to RISC-V bytecode for zero-knowledge execution:
- Contract state managed through `MarketState` struct
- Actions include: `Initialize`, `CreateMarket`, `PlaceBet`, `ResolveMarket`, `ClaimWinnings`
- Initial user balance: 10,000 units
- Admin-controlled market resolution

### Bot Architecture
- SQLite database for local state (`bot/bot.db`)
- Commands: `/start`, `/newbet`, `/bet`, `/mybalance`, `/solve`
- Claude AI integration for market resolution
- Async command handlers using teloxide

### Server Components
- REST API routes in `server/src/app.rs`
- Configuration management via `config.toml` and environment variables (prefix: `HYLE_`)
- Contract initialization and transaction handling
- RISC0 proof generation and verification

## Key Files

- `contracts/contract1/src/lib.rs` - Core contract logic and state management
- `bot/src/main.rs` - Bot command handlers and Telegram integration
- `server/src/app.rs` - REST API endpoints
- `contracts/build.rs` - RISC0 contract compilation script
- `config.toml` - Runtime configuration (ports, contract name, etc.)

## Configuration

The system uses hierarchical configuration:
1. Defaults in `server/src/conf_defaults.toml`
2. Overrides in `config.toml`
3. Environment variables (prefix: `HYLE_`)

Key settings:
- `contract_name`: "contract1"
- `rest_server_port`: 4001
- `da_read_from`: Data availability layer endpoint
- `max_txs_per_proof`: Batching limit for proofs

## Database Schema (Bot)

```sql
users (user_id, username, balance, created_at)
bets (bet_id, creator_id, description, created_at, status)
wagers (wager_id, bet_id, user_id, amount, side, created_at)
solutions (solution_id, bet_id, solver_id, message_id, created_at)
user_init_status (user_id, initialized, initialized_at)
```

## Development Notes

- RISC0 patches applied to RustCrypto for zkVM compatibility
- Reproducible builds use Docker for consistent contract compilation
- Contract metadata generated at build time in `contracts/metadata.rs`
- Async/await throughout with Tokio runtime
- Error handling with `anyhow` and custom `AppError` types