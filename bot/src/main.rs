use anyhow::Result;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::types::ChatKind;
use std::sync::Arc;

mod db;
use db::Database;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "Initialize balance for all users in the group")]
    Init,
    #[command(description = "Create a new bet: /new <title> <description>")]
    New(String),
    #[command(description = "Bet on an existing bet: /bet <title> <yes/no> <amount>")]
    Bet(String),
    #[command(description = "Solve a bet (reply to a message)")]
    Solve,
    #[command(description = "Show the top users by balance")]
    Leaderboard,
    #[command(description = "Reset the entire database (admin only)")]
    Reset,
    #[command(description = "Show help")]
    Help,
}

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn handle_init(bot: Bot, msg: Message, db: Arc<Database>) -> HandlerResult {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /init in chat {}", username, user_id, chat_id.0);
    
    if !matches!(msg.chat.kind, ChatKind::Public(_)) {
        bot.send_message(chat_id, "This command only works in group chats.")
            .await?;
        return Ok(());
    }
    
    // Check if user already initialized their balance
    if db.is_user_initialized(user_id).await? {
        bot.send_message(chat_id, "You have already initialized your balance. You can only use /init once.")
            .await?;
        return Ok(());
    }
    
    // Initialize the user's balance
    if let Some(from) = msg.from.as_ref() {
        let username = from.username.clone();
        db.create_or_update_user(from.id.0 as i64, username, 10000).await?;
        db.mark_user_initialized(from.id.0 as i64).await?;
        bot.send_message(chat_id, format!("✅ Your balance has been initialized to 10,000. This is a one-time initialization."))
            .await?;
        log::info!("Successfully initialized balance for user {}", user_id);
    }
    
    Ok(())
}

async fn handle_new(bot: Bot, msg: Message, db: Arc<Database>, args: String) -> HandlerResult {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /new in chat {} with: {}", username, user_id, chat_id.0, args);
    
    // Parse title and description
    let parts: Vec<&str> = args.splitn(2, ' ').collect();
    if parts.len() < 2 {
        bot.send_message(chat_id, "Usage: /new <title> <description>\nExample: /new weather_tomorrow It will rain tomorrow")
            .await?;
        return Ok(());
    }
    
    let title = parts[0].to_string();
    let description = parts[1].to_string();
    
    if title.is_empty() || description.is_empty() {
        bot.send_message(chat_id, "Both title and description are required. Usage: /new <title> <description>")
            .await?;
        return Ok(());
    }
    
    // Check if user has balance
    let user = db.get_user(user_id).await?;
    if user.is_none() {
        bot.send_message(chat_id, "You need to use /init first to get your initial balance of 10,000.")
            .await?;
        return Ok(());
    }
    
    let bet_id = db.create_bet(user_id, title.clone(), description.clone()).await?;
    
    bot.send_message(
        chat_id,
        format!("✅ Bet #{} created by @{}\n📝 Title: {}\n📄 Description: {}", bet_id, username, title, description)
    )
    .await?;
    log::info!("Bet #{} created successfully by user {} with title: {}", bet_id, user_id, title);
    
    Ok(())
}

async fn handle_bet(bot: Bot, msg: Message, db: Arc<Database>, args: String) -> HandlerResult {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /bet in chat {} with: {}", username, user_id, chat_id.0, args);
    
    // Parse title, yes/no, and amount
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 3 {
        bot.send_message(chat_id, "Usage: /bet <title> <yes/no> <amount>\nExample: /bet weather_tomorrow yes 100")
            .await?;
        return Ok(());
    }
    
    let title = parts[0];
    let side_str = parts[1].to_lowercase();
    let amount_str = parts[2];
    
    // Parse side (yes/no to boolean)
    let side = match side_str.as_str() {
        "yes" | "y" => true,
        "no" | "n" => false,
        _ => {
            bot.send_message(chat_id, "Please specify 'yes' or 'no' for the side.\nUsage: /bet <title> <yes/no> <amount>")
                .await?;
            return Ok(());
        }
    };
    
    let amount = match amount_str.parse::<i64>() {
        Ok(amt) if amt > 0 => amt,
        _ => {
            bot.send_message(chat_id, "Invalid amount. Please provide a positive number.")
                .await?;
            return Ok(());
        }
    };
    
    // Check if user has balance
    let user = db.get_user(user_id).await?;
    let user = match user {
        Some(u) => u,
        None => {
            bot.send_message(chat_id, "You need to use /init first to get your initial balance of 10,000.")
                .await?;
            return Ok(());
        }
    };
    
    if user.balance < amount {
        bot.send_message(chat_id, format!("Insufficient balance. You have {} but tried to bet {}.", user.balance, amount))
            .await?;
        return Ok(());
    }
    
    // Find the bet by title
    let bet = db.find_bet_by_title(title).await?;
    let bet = match bet {
        Some(b) => b,
        None => {
            bot.send_message(chat_id, format!("No open bet found with title '{}'. Use /new to create a bet first.", title))
                .await?;
            return Ok(());
        }
    };
    
    // Create the wager and update balance
    let wager_id = db.create_wager(bet.bet_id, user_id, amount, side).await?;
    let new_balance = user.balance - amount;
    db.update_user_balance(user_id, new_balance).await?;
    
    let side_text = if side { "YES ✅" } else { "NO ❌" };
    
    bot.send_message(
        chat_id,
        format!(
            "💰 Wager placed!\n📝 Bet: {}\n🎯 Side: {}\n💵 Amount: {}\n💳 Remaining balance: {}\n🎲 Wager ID: #{}",
            title, side_text, amount, new_balance, wager_id
        )
    )
    .await?;
    log::info!("Wager #{} placed by user {} on bet {} for amount {} on side {}", wager_id, user_id, bet.bet_id, amount, if side { "yes" } else { "no" });
    
    Ok(())
}

async fn handle_solve(bot: Bot, msg: Message, db: Arc<Database>) -> HandlerResult {
    let chat_id = msg.chat.id;
    let solver_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let solver_username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /solve in chat {}", solver_username, solver_id, chat_id.0);
    
    if msg.reply_to_message().is_none() {
        bot.send_message(chat_id, "Please reply to a message to use /solve")
            .await?;
        return Ok(());
    }
    
    // Check if user has balance
    let user = db.get_user(solver_id).await?;
    if user.is_none() {
        bot.send_message(chat_id, "You need to use /init first to get your initial balance of 10,000.")
            .await?;
        return Ok(());
    }
    
    let replied_msg = msg.reply_to_message().unwrap();
    let message_id = replied_msg.id.0 as i64;
    
    // Extract the text content of the replied message
    let replied_text = replied_msg.text().unwrap_or("<no text content>");
    let replied_user = replied_msg.from.as_ref()
        .and_then(|u| u.username.clone())
        .unwrap_or_else(|| "unknown".to_string());
    
    let bet_id = 1; // TODO: Parse bet_id from the replied message
    
    let solution_id = db.create_solution(bet_id, solver_id, message_id).await?;
    
    bot.send_message(
        chat_id,
        format!(
            "Solution #{} recorded by @{} for bet #{}\n\n📌 Replied to message from @{}:\n\"{}\"",
            solution_id,
            solver_username,
            bet_id,
            replied_user,
            replied_text
        )
    )
    .await?;
    log::info!("Solution #{} created successfully by user {} for bet #{}, replied to: \"{}\"", solution_id, solver_id, bet_id, replied_text);
    
    Ok(())
}

async fn handle_leaderboard(bot: Bot, msg: Message, db: Arc<Database>) -> HandlerResult {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /leaderboard in chat {}", username, user_id, chat_id.0);
    
    // Get top 10 users
    let users = db.get_leaderboard(10).await?;
    
    if users.is_empty() {
        bot.send_message(chat_id, "No users have initialized their balance yet. Use /init to get started!")
            .await?;
        return Ok(());
    }
    
    let mut leaderboard_text = "🏆 LEADERBOARD 🏆\n\n".to_string();
    
    for (index, user) in users.iter().enumerate() {
        let position = index + 1;
        let medal = match position {
            1 => "🥇",
            2 => "🥈",
            3 => "🥉",
            _ => "  ",
        };
        
        let username_display = user.username.as_ref()
            .map(|u| format!("@{}", u))
            .unwrap_or_else(|| format!("User {}", user.user_id));
        
        leaderboard_text.push_str(&format!(
            "{} #{}: {} - {} coins\n",
            medal, position, username_display, user.balance
        ));
    }
    
    bot.send_message(chat_id, leaderboard_text)
        .await?;
    
    Ok(())
}

async fn handle_reset(bot: Bot, msg: Message, db: Arc<Database>) -> HandlerResult {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /reset in chat {}", username, user_id, chat_id.0);
    
    // Check if it's a group chat
    if matches!(msg.chat.kind, ChatKind::Public(_)) {
        // In group chats, only admins can reset
        let admins = bot.get_chat_administrators(chat_id).await?;
        let is_admin = admins.iter().any(|admin| {
            admin.user.id.0 as i64 == user_id
        });
        
        if !is_admin {
            bot.send_message(chat_id, "Only admins can use the /reset command in group chats.")
                .await?;
            return Ok(());
        }
    }
    
    // Reset the database
    db.reset_all().await?;
    
    bot.send_message(
        chat_id,
        "⚠️ Database has been reset!\n\n• All user balances cleared\n• All bets removed\n• All solutions deleted\n• All users need to /init again"
    )
    .await?;
    
    log::info!("Database reset successfully by user {}", user_id);
    
    Ok(())
}

async fn handle_message(bot: Bot, msg: Message, cmd: Command, db: Arc<Database>) -> HandlerResult {
    match cmd {
        Command::Init => handle_init(bot, msg, db).await,
        Command::New(args) => handle_new(bot, msg, db, args).await,
        Command::Bet(args) => handle_bet(bot, msg, db, args).await,
        Command::Solve => handle_solve(bot, msg, db).await,
        Command::Leaderboard => handle_leaderboard(bot, msg, db).await,
        Command::Reset => handle_reset(bot, msg, db).await,
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    log::info!("Starting bot...");
    
    let database_url = "sqlite://bot.db?mode=rwc";
    let db = Arc::new(Database::new(database_url).await?);
    db.init().await?;
    log::info!("Database initialized");
    
    let bot = Bot::from_env();
    
    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
            let db = Arc::clone(&db);
            async move {
                if let Err(e) = handle_message(bot, msg, cmd, db).await {
                    log::error!("Error handling message: {:?}", e);
                }
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            }
        });
    
    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    
    Ok(())
}