# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based Telegram bot project using the Teloxide framework. Currently in early development stage with minimal implementation.

## Development Commands

### Build
```bash
cargo build
```

### Run
```bash
cargo run
```

### Test
```bash
cargo test
```

### Check code without building
```bash
cargo check
```

### Format code
```bash
cargo fmt
```

### Lint code
```bash
cargo clippy
```

## Architecture

The project is a Telegram bot application built with:
- **Teloxide**: Modern Telegram Bot API framework for Rust with macro support
- **Tokio**: Async runtime with multi-threaded support
- **Logging**: Configured with `log` and `pretty_env_logger` for debugging

The entry point is in `src/main.rs`. As development progresses, bot handlers, commands, and state management will likely be organized into separate modules under the `src/` directory.

## Dependencies

Key dependencies from `Cargo.toml`:
- `teloxide` (0.17.0) - Telegram bot framework with macros feature enabled
- `tokio` (1.8) - Async runtime with rt-multi-thread and macros features
- `log` (0.4) and `pretty_env_logger` (0.5) - Logging infrastructure