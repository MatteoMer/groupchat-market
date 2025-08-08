use anyhow::Result;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use teloxide::types::ChatKind;
use std::sync::Arc;

mod db;
mod claude;
mod api_client;
use db::Database;
use api_client::MarketApiClient;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "Initialize balance for all users in the group")]
    Init,
    #[command(description = "Create a new bet: /new <description>")]
    New(String),
    #[command(description = "Bet on an existing bet: /bet <bet_id> <yes/no> <amount>")]
    Bet(String),
    #[command(description = "List all bets")]
    List,
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

struct BotContext {
    db: Arc<Database>,
    api_client: Arc<MarketApiClient>,
    contract_name: String,
}

async fn handle_init(bot: Bot, msg: Message, ctx: Arc<BotContext>) -> HandlerResult {
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
    if ctx.db.is_user_initialized(user_id).await? {
        bot.send_message(chat_id, "You have already initialized your balance. You can only use /init once.")
            .await?;
        return Ok(());
    }
    
    // Initialize the user's balance on the blockchain
    if let Some(from) = msg.from.as_ref() {
        let username = from.username.clone();
        
        // Call the blockchain API to initialize the user
        match ctx.api_client.initialize_user(user_id.to_string(), &ctx.contract_name).await {
            Ok(tx_hash) => {
                // Record initialization in local database
                ctx.db.create_or_update_user(from.id.0 as i64, username, 10000).await?;
                ctx.db.mark_user_initialized(from.id.0 as i64).await?;
                bot.send_message(chat_id, format!("✅ Your balance has been initialized to 10,000 on-chain.\nTransaction: {}", tx_hash))
                    .await?;
                log::info!("Successfully initialized balance for user {} with tx {}", user_id, tx_hash);
            }
            Err(e) => {
                bot.send_message(chat_id, format!("❌ Failed to initialize balance: {}", e))
                    .await?;
                log::error!("Failed to initialize user {}: {}", user_id, e);
            }
        }
    }
    
    Ok(())
}

async fn handle_new(bot: Bot, msg: Message, ctx: Arc<BotContext>, description: String) -> HandlerResult {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /new in chat {} with: {}", username, user_id, chat_id.0, description);
    
    if description.trim().is_empty() {
        bot.send_message(chat_id, "Usage: /new <description>\nExample: /new Will it rain tomorrow?")
            .await?;
        return Ok(());
    }
    
    // Check if user has balance
    let user = ctx.db.get_user(user_id).await?;
    if user.is_none() {
        bot.send_message(chat_id, "You need to use /init first to get your initial balance of 10,000.")
            .await?;
        return Ok(());
    }
    
    // Create market on blockchain
    match ctx.api_client.create_market(user_id.to_string(), description.clone(), &ctx.contract_name).await {
        Ok(tx_hash) => {
            // Store in local database for tracking
            let bet_id = ctx.db.create_bet(user_id, description.clone()).await?;
            
            bot.send_message(
                chat_id,
                format!("✅ Market #{} created on-chain by @{}\n📄 Description: {}\nTransaction: {}", 
                    bet_id, username, description, tx_hash)
            )
            .await?;
            log::info!("Market #{} created successfully by user {} with tx {}", bet_id, user_id, tx_hash);
        }
        Err(e) => {
            bot.send_message(chat_id, format!("❌ Failed to create market: {}", e))
                .await?;
            log::error!("Failed to create market for user {}: {}", user_id, e);
        }
    }
    
    Ok(())
}

async fn handle_bet(bot: Bot, msg: Message, ctx: Arc<BotContext>, args: String) -> HandlerResult {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /bet in chat {} with: {}", username, user_id, chat_id.0, args);
    
    // Parse bet_id, yes/no, and amount
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 3 {
        bot.send_message(chat_id, "Usage: /bet <bet_id> <yes/no> <amount>\nExample: /bet 1 yes 100")
            .await?;
        return Ok(());
    }
    
    let bet_id = match parts[0].parse::<i64>() {
        Ok(id) => id,
        Err(_) => {
            bot.send_message(chat_id, "Invalid bet ID. Please provide a number.\nUsage: /bet <bet_id> <yes/no> <amount>")
                .await?;
            return Ok(());
        }
    };
    
    let side_str = parts[1].to_lowercase();
    let amount_str = parts[2];
    
    // Parse side (yes/no to boolean)
    let side = match side_str.as_str() {
        "yes" | "y" => true,
        "no" | "n" => false,
        _ => {
            bot.send_message(chat_id, "Please specify 'yes' or 'no' for the side.\nUsage: /bet <bet_id> <yes/no> <amount>")
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
    let user = ctx.db.get_user(user_id).await?;
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
    
    // Find the bet by ID
    let bet = ctx.db.get_bet_by_id(bet_id).await?;
    let bet = match bet {
        Some(b) if b.status == "open" => b,
        Some(_) => {
            bot.send_message(chat_id, format!("Bet #{} is already closed.", bet_id))
                .await?;
            return Ok(());
        }
        None => {
            bot.send_message(chat_id, format!("Bet #{} not found. Use /list to see available bets.", bet_id))
                .await?;
            return Ok(());
        }
    };
    
    // Place bet on blockchain
    match ctx.api_client.place_bet(user_id.to_string(), bet_id as u64, side, amount as u128, &ctx.contract_name).await {
        Ok(tx_hash) => {
            // Create the wager and update balance locally
            let _wager_id = ctx.db.create_wager(bet.bet_id, user_id, amount, side).await?;
            let new_balance = user.balance - amount;
            ctx.db.update_user_balance(user_id, new_balance).await?;
            
            let side_text = if side { "YES ✅" } else { "NO ❌" };
            
            bot.send_message(
                chat_id,
                format!(
                    "💰 Bet placed on-chain!\n📝 Market #{}: {}\n🎯 Side: {}\n💵 Amount: {}\n💳 Remaining balance: {}\nTransaction: {}",
                    bet_id, bet.description, side_text, amount, new_balance, tx_hash
                )
            )
            .await?;
            log::info!("Bet placed by user {} on market {} for amount {} on side {} with tx {}", 
                user_id, bet.bet_id, amount, if side { "yes" } else { "no" }, tx_hash);
        }
        Err(e) => {
            bot.send_message(chat_id, format!("❌ Failed to place bet: {}", e))
                .await?;
            log::error!("Failed to place bet for user {}: {}", user_id, e);
        }
    }
    
    Ok(())
}

async fn handle_solve(bot: Bot, msg: Message, ctx: Arc<BotContext>) -> HandlerResult {
    let chat_id = msg.chat.id;
    let solver_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let solver_username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /solve in chat {}", solver_username, solver_id, chat_id.0);
    
    // Parse optional bet_id from command
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().collect();
    let bet_id = if parts.len() > 1 {
        parts[1].parse::<i64>().ok()
    } else {
        None
    };
    
    if msg.reply_to_message().is_none() {
        bot.send_message(chat_id, "Please reply to a message to use /solve\nUsage: /solve [bet_id]")
            .await?;
        return Ok(());
    }
    
    // Check if user has balance
    let user = ctx.db.get_user(solver_id).await?;
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
    
    // If no bet_id provided, we need to ask for it
    let bet_id = match bet_id {
        Some(id) => id,
        None => {
            bot.send_message(
                chat_id,
                "Please specify which bet this solves. Usage: /solve <bet_id>\nExample: /solve 1"
            )
            .await?;
            return Ok(());
        }
    };
    
    // Get the bet details
    let bet = match ctx.db.get_bet_by_id(bet_id).await? {
        Some(b) if b.status == "open" => b,
        Some(_) => {
            bot.send_message(chat_id, "This bet is already closed.")
                .await?;
            return Ok(());
        }
        None => {
            bot.send_message(chat_id, format!("Bet #{} not found.", bet_id))
                .await?;
            return Ok(());
        }
    };
    
    // Get Claude API key from environment
    let api_key = match std::env::var("CLAUDE_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            log::error!("CLAUDE_API_KEY not set");
            bot.send_message(chat_id, "❌ Bot configuration error: Claude API key not set.")
                .await?;
            return Ok(());
        }
    };
    
    // Send processing message
    bot.send_message(chat_id, "🤔 Evaluating solution with Claude AI...")
        .await?;
    
    // Call Claude to evaluate the solution
    let resolution = match claude::evaluate_bet_resolution(
        &api_key,
        bet_id,
        &bet.description,
        replied_text,
        &replied_user,
    ).await {
        Ok(res) => res,
        Err(e) => {
            log::error!("Claude API error: {:?}", e);
            bot.send_message(
                chat_id,
                format!("❌ Failed to evaluate solution: {}", e)
            )
            .await?;
            return Ok(());
        }
    };
    
    // Record the solution
    let solution_id = ctx.db.create_solution(bet_id, solver_id, message_id).await?;
    
    if resolution.resolved {
        // Resolve the market on blockchain
        match ctx.api_client.resolve_market(
            solver_id.to_string(),
            bet_id as u64,
            resolution.outcome,
            &ctx.contract_name
        ).await {
            Ok(tx_hash) => {
                // Close the bet locally
                ctx.db.close_bet(bet_id, resolution.outcome).await?;
                
                // The contract automatically distributes winnings when resolving
                // Log that resolution was successful but don't update balances locally
                log::info!("Market #{} resolved. Contract automatically distributed winnings to winners.", bet_id);
                
                // Note: In a production system, you might want to:
                // 1. Query the blockchain for updated balances
                // 2. Update local database with the new balances
                // This ensures local state stays in sync with on-chain state
                
                bot.send_message(
                    chat_id,
                    format!(
                        "✅ MARKET RESOLVED ON-CHAIN!\n\n📊 Market #{}\n📄 Description: {}\n💬 Solution: \"{}\"\n👤 Solved by: @{}\n🎯 Outcome: {}\n\n🤖 Claude's analysis: {}\n\nTransaction: {}\n\n💰 Winnings have been automatically distributed to all winners!",
                        bet_id,
                        bet.description,
                        replied_text,
                        solver_username,
                        if resolution.outcome { "YES ✅" } else { "NO ❌" },
                        resolution.reasoning,
                        tx_hash
                    )
                )
                .await?;
                log::info!("Market #{} resolved on-chain with tx {}", bet_id, tx_hash);
            }
            Err(e) => {
                bot.send_message(
                    chat_id,
                    format!("❌ Failed to resolve market on-chain: {}\n\nThe bet remains open.", e)
                )
                .await?;
                log::error!("Failed to resolve market {}: {}", bet_id, e);
            }
        }
    } else {
        bot.send_message(
            chat_id,
            format!(
                "❌ NOT RESOLVED\n\n📊 Market #{}\n📄 Description: {}\n💬 Proposed solution: \"{}\"\n👤 Proposed by: @{}\n\n🤖 Claude's analysis: {}\n\nThe market remains open.",
                bet_id,
                bet.description,
                replied_text,
                solver_username,
                resolution.reasoning
            )
        )
        .await?;
    }
    
    log::info!("Solution #{} evaluated for bet #{}: resolved={}", solution_id, bet_id, resolution.resolved);
    
    Ok(())
}

async fn handle_list(bot: Bot, msg: Message, ctx: Arc<BotContext>) -> HandlerResult {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /list in chat {}", username, user_id, chat_id.0);
    
    let bets = ctx.db.get_all_bets().await?;
    
    if bets.is_empty() {
        bot.send_message(chat_id, "No bets available. Use /new to create the first bet!")
            .await?;
        return Ok(());
    }
    
    let mut message = "📄 **AVAILABLE BETS** 📄\n\n".to_string();
    
    for bet in bets.iter().take(20) {  // Limit to 20 most recent bets
        let status_emoji = match bet.status.as_str() {
            "open" => "🟢",
            "resolved_yes" => "✅",
            "resolved_no" => "❌",
            _ => "❔",
        };
        
        let truncated_desc = if bet.description.len() > 50 {
            format!("{}...", &bet.description[..50])
        } else {
            bet.description.clone()
        };
        
        message.push_str(&format!(
            "{} Bet #{}: {}\n",
            status_emoji, bet.bet_id, truncated_desc
        ));
    }
    
    if bets.len() > 20 {
        message.push_str(&format!("\n... and {} more bets", bets.len() - 20));
    }
    
    message.push_str("\n\nUse /bet <bet_id> <yes/no> <amount> to place a wager!");
    
    bot.send_message(chat_id, message)
        .await?;
    
    Ok(())
}

async fn handle_leaderboard(bot: Bot, msg: Message, ctx: Arc<BotContext>) -> HandlerResult {
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64).unwrap_or(0);
    let username = msg.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| "unknown".to_string());
    
    log::info!("User @{} (ID: {}) called /leaderboard in chat {}", username, user_id, chat_id.0);
    
    // Get top 10 users
    let users = ctx.db.get_leaderboard(10).await?;
    
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

async fn handle_reset(bot: Bot, msg: Message, ctx: Arc<BotContext>) -> HandlerResult {
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
    ctx.db.reset_all().await?;
    
    bot.send_message(
        chat_id,
        "⚠️ Database has been reset!\n\n• All user balances cleared\n• All bets removed\n• All solutions deleted\n• All users need to /init again"
    )
    .await?;
    
    log::info!("Database reset successfully by user {}", user_id);
    
    Ok(())
}

async fn handle_message(bot: Bot, msg: Message, cmd: Command, ctx: Arc<BotContext>) -> HandlerResult {
    match cmd {
        Command::Init => handle_init(bot, msg, ctx).await,
        Command::New(args) => handle_new(bot, msg, ctx, args).await,
        Command::Bet(args) => handle_bet(bot, msg, ctx, args).await,
        Command::List => handle_list(bot, msg, ctx).await,
        Command::Solve => handle_solve(bot, msg, ctx).await,
        Command::Leaderboard => handle_leaderboard(bot, msg, ctx).await,
        Command::Reset => handle_reset(bot, msg, ctx).await,
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
    
    // Initialize database
    let database_url = "sqlite://bot.db?mode=rwc";
    let db = Arc::new(Database::new(database_url).await?);
    db.init().await?;
    log::info!("Database initialized");
    
    // Get server URL from environment or use default
    let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| "http://localhost:4001".to_string());
    log::info!("Connecting to server at: {}", server_url);
    
    // Initialize API client
    let api_client = Arc::new(MarketApiClient::new(server_url.clone()));
    
    // Check server health
    match api_client.health_check().await {
        Ok(true) => log::info!("Server is healthy"),
        Ok(false) => log::warn!("Server health check returned false"),
        Err(e) => {
            log::error!("Failed to connect to server: {}. Bot will run in offline mode.", e);
            // You could exit here if you want to require server connection
            // return Err(anyhow::anyhow!("Server not available"));
        }
    }
    
    // Get contract name from server
    let contract_name = match api_client.get_config().await {
        Ok(config) => {
            log::info!("Got contract name from server: {}", config.contract_name);
            config.contract_name
        }
        Err(e) => {
            log::warn!("Failed to get config from server: {}. Using default.", e);
            "contract1".to_string()
        }
    };
    
    // Create bot context
    let ctx = Arc::new(BotContext {
        db,
        api_client,
        contract_name,
    });
    
    let bot = Bot::from_env();
    
    let handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
            let ctx = Arc::clone(&ctx);
            async move {
                if let Err(e) = handle_message(bot, msg, cmd, ctx).await {
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