use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub user_id: i64,
    pub username: Option<String>,
    pub balance: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bet {
    pub bet_id: i64,
    pub creator_id: i64,
    pub description: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Wager {
    pub wager_id: i64,
    pub bet_id: i64,
    pub user_id: i64,
    pub amount: i64,
    pub side: bool, // true = yes, false = no
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Solution {
    pub solution_id: i64,
    pub bet_id: i64,
    pub solver_id: i64,
    pub message_id: i64,
    pub created_at: String,
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        use sqlx::sqlite::SqliteConnectOptions;
        use std::str::FromStr;
        
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true);
        
        let pool = SqlitePool::connect_with(options).await?;
        Ok(Self { pool })
    }

    pub async fn init(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                user_id INTEGER PRIMARY KEY,
                username TEXT,
                balance INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS bets (
                bet_id INTEGER PRIMARY KEY AUTOINCREMENT,
                creator_id INTEGER NOT NULL,
                description TEXT NOT NULL,
                created_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'open',
                FOREIGN KEY (creator_id) REFERENCES users(user_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS wagers (
                wager_id INTEGER PRIMARY KEY AUTOINCREMENT,
                bet_id INTEGER NOT NULL,
                user_id INTEGER NOT NULL,
                amount INTEGER NOT NULL,
                side BOOLEAN NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (bet_id) REFERENCES bets(bet_id),
                FOREIGN KEY (user_id) REFERENCES users(user_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS solutions (
                solution_id INTEGER PRIMARY KEY AUTOINCREMENT,
                bet_id INTEGER NOT NULL,
                solver_id INTEGER NOT NULL,
                message_id INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (bet_id) REFERENCES bets(bet_id),
                FOREIGN KEY (solver_id) REFERENCES users(user_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_init_status (
                user_id INTEGER PRIMARY KEY,
                initialized BOOLEAN NOT NULL DEFAULT TRUE,
                initialized_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn create_or_update_user(&self, user_id: i64, username: Option<String>, balance: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO users (user_id, username, balance, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(user_id) DO UPDATE SET
                username = excluded.username,
                balance = excluded.balance
            "#,
        )
        .bind(user_id)
        .bind(username)
        .bind(balance)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_user(&self, user_id: i64) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT user_id, username, balance, created_at FROM users WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    pub async fn create_bet(&self, creator_id: i64, description: String) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            r#"
            INSERT INTO bets (creator_id, description, created_at, status)
            VALUES (?1, ?2, ?3, 'open')
            "#,
        )
        .bind(creator_id)
        .bind(description)
        .bind(now)
        .execute(&self.pool)
        .await?;
        
        Ok(result.last_insert_rowid())
    }

    pub async fn create_wager(&self, bet_id: i64, user_id: i64, amount: i64, side: bool) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            r#"
            INSERT INTO wagers (bet_id, user_id, amount, side, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )
        .bind(bet_id)
        .bind(user_id)
        .bind(amount)
        .bind(side)
        .bind(now)
        .execute(&self.pool)
        .await?;
        
        Ok(result.last_insert_rowid())
    }

    pub async fn get_all_bets(&self) -> Result<Vec<Bet>> {
        let bets = sqlx::query_as::<_, Bet>(
            "SELECT bet_id, creator_id, description, created_at, status FROM bets ORDER BY bet_id DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(bets)
    }

    pub async fn get_bet_by_id(&self, bet_id: i64) -> Result<Option<Bet>> {
        let bet = sqlx::query_as::<_, Bet>(
            "SELECT bet_id, creator_id, description, created_at, status FROM bets WHERE bet_id = ?",
        )
        .bind(bet_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(bet)
    }

    pub async fn close_bet(&self, bet_id: i64, resolution: bool) -> Result<()> {
        let status = if resolution { "resolved_yes" } else { "resolved_no" };
        sqlx::query(
            "UPDATE bets SET status = ? WHERE bet_id = ?",
        )
        .bind(status)
        .bind(bet_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_user_balance(&self, user_id: i64, new_balance: i64) -> Result<()> {
        sqlx::query(
            "UPDATE users SET balance = ? WHERE user_id = ?",
        )
        .bind(new_balance)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_leaderboard(&self, limit: i64) -> Result<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            "SELECT user_id, username, balance, created_at FROM users ORDER BY balance DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    pub async fn create_solution(&self, bet_id: i64, solver_id: i64, message_id: i64) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            r#"
            INSERT INTO solutions (bet_id, solver_id, message_id, created_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(bet_id)
        .bind(solver_id)
        .bind(message_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        
        Ok(result.last_insert_rowid())
    }

    pub async fn is_user_initialized(&self, user_id: i64) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT initialized FROM user_init_status WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(result.unwrap_or(false))
    }

    pub async fn mark_user_initialized(&self, user_id: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT INTO user_init_status (user_id, initialized, initialized_at)
            VALUES (?1, TRUE, ?2)
            ON CONFLICT(user_id) DO NOTHING
            "#,
        )
        .bind(user_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn reset_all(&self) -> Result<()> {
        sqlx::query("DELETE FROM solutions")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("DELETE FROM wagers")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("DELETE FROM bets")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("DELETE FROM users")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("DELETE FROM user_init_status")
            .execute(&self.pool)
            .await?;
        
        // Reset autoincrement counters
        sqlx::query("DELETE FROM sqlite_sequence WHERE name IN ('bets', 'solutions', 'wagers')")
            .execute(&self.pool)
            .await?;
        
        Ok(())
    }
}