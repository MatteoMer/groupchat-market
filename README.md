# Groupchat Market: Prediction markets for your groupchat

Built during [Frontiers](https://frontiers.paradigm.xyz/)

**Turn your chats into prediction markets**

---

### Example

```
Alice: "I bet John will be late to the meeting again"
Bob: /new Will John arrive on time to the 3pm meeting?
Bot: ✅ Market #1 created: Will John arrive on time to the 3pm meeting?
Alice: /bet 1 no 500
Bob: /bet 1 yes 100
Charlie: /bet 1 no 200

[Later at 3:05pm]
John: "Sorry guys, traffic was terrible"
Alice: /solve 1 [replying to John's message]
Bot: ✅ MARKET RESOLVED: NO wins
     🤖 AI Analysis: "John apologized for being late, confirming he did not arrive on time"
     💰 Payouts: Alice +750, Charlie +300
```

---

### Architecture

Everything: bot, server and the contract are written in Rust. The contract is a vApp proven on a zkVM. 

This proof is used for settlement on [Hyli](https://hyli.org/), a blockchain where every app is a vApp and where the execution is offchain, the consensus is only verifying the proof

---

### Getting Started

#### Prerequisites
- Rust toolchain (latest stable)
- Telegram Bot Token (from [@BotFather](https://t.me/botfather))
- Claude API Key (from [Anthropic](https://console.anthropic.com/))
- Hyli node (see [Hyli docs](https://docs.hyli.org/) for setup)

#### 1. Run the Hyli Node
Follow the official documentation at [docs.hyli.org](https://docs.hyli.org/) to set up and run a Hyli node.

#### 2. Build the Contract
```bash
# Clone the repository
git clone https://github.com/MatteoMer/groupchat-market.git
cd groupchat-market

# Build the contract (fast/local build)
cd contracts
cargo build --features nonreproducible,contract1

# Or for reproducible build (slower, uses Docker)
cargo build --features build,contract1
```

#### 3. Run the Server
```bash
# From project root
RISC0_DEV_MODE=1 SP1_PROVER=mock cargo run -p server

# Server will start on port 4001 by default
```

#### 4. Run the Telegram Bot
```bash
# Set required environment variables
export TELOXIDE_TOKEN="your_telegram_bot_token"
export CLAUDE_API_KEY="your_claude_api_key"

# Run the bot
cargo run -p bot

# The bot will connect to the server at localhost:4001
```

#### Configuration
- Edit `config.toml` to customize ports and settings
- Server config can be overridden with `HYLE_` prefixed environment variables
- Bot database is stored in `bot/bot.db`

## License

MIT
