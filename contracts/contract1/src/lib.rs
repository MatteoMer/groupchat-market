use borsh::{io::Error, BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use sdk::{ContractName, Identity, RunResult};

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "client")]
pub mod indexer;

impl sdk::FullStateRevert for ParimutuelMarket {}

impl sdk::ZkContract for ParimutuelMarket {
    /// Entry point of the contract's logic
    fn execute(&mut self, calldata: &sdk::Calldata) -> RunResult {
        // Parse contract inputs
        let (action, ctx) = sdk::utils::parse_calldata::<Nonced<MarketAction>>(calldata)?;
        let identity = calldata.identity.clone();

        // Execute the given action
        let res = match action.action {
            MarketAction::SetAdmin { new_admin } => self.set_admin(identity, new_admin)?,
            MarketAction::Initialize => self.initialize(identity)?,
            MarketAction::CreateMarket { description } => {
                self.create_market(identity, description)?
            }
            MarketAction::PlaceBet { market_id, side, amount } => {
                self.place_bet(identity, market_id, side, amount)?
            }
            MarketAction::ResolveMarket { market_id, outcome } => {
                self.resolve_market(identity, market_id, outcome)?
            }
            MarketAction::ClaimWinnings { market_id } => {
                self.claim_winnings(identity, market_id)?
            }
            MarketAction::GetBalance => self.get_balance(identity)?,
            MarketAction::GetMarketInfo { market_id } => self.get_market_info(market_id)?,
        };

        Ok((res.into_bytes(), ctx, vec![]))
    }

    /// Serialize the full state on-chain
    fn commit(&self) -> sdk::StateCommitment {
        sdk::StateCommitment(self.as_bytes().expect("Failed to encode ParimutuelMarket"))
    }
}

impl ParimutuelMarket {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            markets: HashMap::new(),
            next_market_id: 1,
            admin: None, // Admin must be set through SetAdmin action
        }
    }
    
    pub fn new_with_admin(admin: Identity) -> Self {
        Self {
            users: HashMap::new(),
            markets: HashMap::new(),
            next_market_id: 1,
            admin: Some(admin),
        }
    }

    fn get_or_create_user(&mut self, identity: Identity) -> &mut UserState {
        self.users.entry(identity).or_insert_with(|| UserState {
            balance: 0,
            initialized: false,
            bets: Vec::new(),
        })
    }
    
    pub fn set_admin(&mut self, identity: Identity, new_admin: Identity) -> Result<String, String> {
        // If no admin is set yet, allow anyone to set it (first-time setup)
        // Otherwise, only current admin can change it
        if let Some(current_admin) = &self.admin {
            if *current_admin != identity {
                return Err("Only current admin can change admin".to_string());
            }
        }
        
        self.admin = Some(new_admin.clone());
        Ok(format!("Admin set to: {}", new_admin.to_string()))
    }

    pub fn initialize(&mut self, identity: Identity) -> Result<String, String> {
        let user = self.get_or_create_user(identity.clone());
        if user.initialized {
            return Err("User already initialized".to_string());
        }
        
        user.balance = INITIAL_BALANCE;
        user.initialized = true;
        
        Ok(format!("Initialized with {} balance", INITIAL_BALANCE))
    }

    pub fn create_market(
        &mut self,
        identity: Identity,
        description: String,
    ) -> Result<String, String> {
        let user = self.users.get(&identity).ok_or("User not initialized")?;
        if !user.initialized {
            return Err("User not initialized. Use Initialize first.".to_string());
        }

        let market_id = self.next_market_id;
        self.next_market_id += 1;

        let market = Market {
            id: market_id,
            creator: identity,
            description,
            yes_pool: 0,
            no_pool: 0,
            yes_bettors: HashMap::new(),
            no_bettors: HashMap::new(),
            status: MarketStatus::Open,
            created_at: 0, // In production, use actual timestamp
        };

        self.markets.insert(market_id, market);
        
        Ok(format!("Market #{} created", market_id))
    }

    pub fn place_bet(
        &mut self,
        identity: Identity,
        market_id: u64,
        side: bool, // true = yes, false = no
        amount: u128,
    ) -> Result<String, String> {
        // Check user has enough balance
        let user = self.users.get_mut(&identity).ok_or("User not initialized")?;
        if !user.initialized {
            return Err("User not initialized. Use Initialize first.".to_string());
        }
        
        if user.balance < amount {
            return Err(format!(
                "Insufficient balance. Have: {}, Need: {}",
                user.balance, amount
            ));
        }

        // Check market exists and is open
        let market = self.markets.get_mut(&market_id)
            .ok_or("Market not found")?;
        
        if market.status != MarketStatus::Open {
            return Err("Market is not open for betting".to_string());
        }

        // Deduct balance and place bet
        user.balance -= amount;
        user.bets.push(UserBet {
            market_id,
            side,
            amount,
            claimed: false,
        });

        // Add to market pools
        if side {
            market.yes_pool += amount;
            *market.yes_bettors.entry(identity).or_insert(0) += amount;
        } else {
            market.no_pool += amount;
            *market.no_bettors.entry(identity).or_insert(0) += amount;
        }

        let side_str = if side { "YES" } else { "NO" };
        Ok(format!(
            "Bet placed: {} on {} for market #{}. Remaining balance: {}",
            amount, side_str, market_id, user.balance
        ))
    }

    pub fn resolve_market(
        &mut self,
        identity: Identity,
        market_id: u64,
        outcome: bool, // true = yes won, false = no won
    ) -> Result<String, String> {
        // Only admin can resolve markets
        match &self.admin {
            Some(admin) if *admin == identity => {},
            Some(_) => return Err("Only admin can resolve markets".to_string()),
            None => return Err("No admin set. Use SetAdmin first.".to_string()),
        }
        
        let market = self.markets.get_mut(&market_id)
            .ok_or("Market not found")?;
        
        if market.status != MarketStatus::Open {
            return Err("Market is not open".to_string());
        }

        market.status = if outcome {
            MarketStatus::ResolvedYes
        } else {
            MarketStatus::ResolvedNo
        };

        let outcome_str = if outcome { "YES" } else { "NO" };
        Ok(format!("Market #{} resolved as {}", market_id, outcome_str))
    }

    pub fn claim_winnings(
        &mut self,
        identity: Identity,
        market_id: u64,
    ) -> Result<String, String> {
        let market = self.markets.get(&market_id)
            .ok_or("Market not found")?;
        
        let (is_resolved, winning_side) = match market.status {
            MarketStatus::ResolvedYes => (true, true),
            MarketStatus::ResolvedNo => (true, false),
            _ => (false, false),
        };
        
        if !is_resolved {
            return Err("Market not resolved yet".to_string());
        }

        let user = self.users.get_mut(&identity)
            .ok_or("User not found")?;
        
        // Find user's bet on this market
        let bet = user.bets.iter_mut()
            .find(|b| b.market_id == market_id && !b.claimed)
            .ok_or("No unclaimed bet found for this market")?;
        
        if bet.side != winning_side {
            bet.claimed = true;
            return Ok("Your bet did not win".to_string());
        }

        // Calculate winnings using parimutuel formula
        let user_stake = if winning_side {
            *market.yes_bettors.get(&identity).unwrap_or(&0)
        } else {
            *market.no_bettors.get(&identity).unwrap_or(&0)
        };
        
        let winning_pool = if winning_side { market.yes_pool } else { market.no_pool };
        let losing_pool = if winning_side { market.no_pool } else { market.yes_pool };
        let total_pool = winning_pool + losing_pool;
        
        if winning_pool == 0 {
            return Err("No winning pool".to_string());
        }
        
        // Payout = (user_stake / winning_pool) * total_pool
        let payout = (user_stake as f64 / winning_pool as f64 * total_pool as f64) as u128;
        
        user.balance += payout;
        bet.claimed = true;
        
        Ok(format!("Claimed {} winnings from market #{}", payout, market_id))
    }

    pub fn get_balance(&self, identity: Identity) -> Result<String, String> {
        let user = self.users.get(&identity)
            .ok_or("User not found")?;
        Ok(format!("Balance: {}", user.balance))
    }

    pub fn get_market_info(&self, market_id: u64) -> Result<String, String> {
        let market = self.markets.get(&market_id)
            .ok_or("Market not found")?;
        
        let status_str = match market.status {
            MarketStatus::Open => "Open",
            MarketStatus::ResolvedYes => "Resolved: YES",
            MarketStatus::ResolvedNo => "Resolved: NO",
        };
        
        Ok(format!(
            "Market #{}: {}\nStatus: {}\nYES pool: {}\nNO pool: {}\nTotal pool: {}",
            market.id,
            market.description,
            status_str,
            market.yes_pool,
            market.no_pool,
            market.yes_pool + market.no_pool
        ))
    }
}

// Constants
const INITIAL_BALANCE: u128 = 10_000;

// Data structures
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserState {
    balance: u128,
    initialized: bool,
    bets: Vec<UserBet>,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct UserBet {
    market_id: u64,
    side: bool, // true = yes, false = no
    amount: u128,
    claimed: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct Market {
    id: u64,
    creator: Identity,
    description: String,
    yes_pool: u128,
    no_pool: u128,
    yes_bettors: HashMap<Identity, u128>,
    no_bettors: HashMap<Identity, u128>,
    status: MarketStatus,
    created_at: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MarketStatus {
    Open,
    ResolvedYes,
    ResolvedNo,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, Default)]
pub struct ParimutuelMarket {
    users: HashMap<Identity, UserState>,
    markets: HashMap<u64, Market>,
    next_market_id: u64,
    admin: Option<Identity>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct Nonced<T> {
    pub action: T,
    pub nonce: u64,
}

/// Enum representing possible calls to the contract functions
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum MarketAction {
    SetAdmin { new_admin: Identity },
    Initialize,
    CreateMarket { description: String },
    PlaceBet { market_id: u64, side: bool, amount: u128 },
    ResolveMarket { market_id: u64, outcome: bool },
    ClaimWinnings { market_id: u64 },
    GetBalance,
    GetMarketInfo { market_id: u64 },
}

impl MarketAction {
    pub fn as_blob(&self, contract_name: sdk::ContractName) -> sdk::Blob {
        sdk::Blob {
            contract_name,
            data: sdk::BlobData(borsh::to_vec(self).expect("Failed to encode MarketAction")),
        }
    }
}

impl ParimutuelMarket {
    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        borsh::to_vec(self)
    }
}

impl From<sdk::StateCommitment> for ParimutuelMarket {
    fn from(state: sdk::StateCommitment) -> Self {
        borsh::from_slice(&state.0)
            .map_err(|_| "Could not decode parimutuel market state".to_string())
            .unwrap()
    }
}