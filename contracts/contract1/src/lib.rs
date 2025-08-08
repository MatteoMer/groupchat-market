use borsh::{io::Error, BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use sdk::{Identity, RunResult};

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "client")]
pub mod indexer;

impl sdk::ZkContract for Contract1 {
    /// Entry point of the contract's logic
    fn execute(&mut self, calldata: &sdk::Calldata) -> RunResult {
        // Parse contract inputs
        let (action, ctx) = sdk::utils::parse_raw_calldata::<MarketAction>(calldata)?;
        let identity = calldata.identity.clone();

        // Execute the given action
        let res = match action {
            MarketAction::SetAdmin { new_admin } => self.set_admin(identity, new_admin)?,
            MarketAction::Initialize {} => self.initialize(identity)?,
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
        sdk::StateCommitment(self.as_bytes().expect("Failed to encode Contract1"))
    }
}

impl Contract1 {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            markets: HashMap::new(),
            next_market_id: 0,
        }
    }
    
    pub fn new_with_admin(_admin: Identity) -> Self {
        // Admin functionality removed - just create a normal instance
        Self::new()
    }

    fn get_or_create_user(&mut self, identity: Identity) -> &mut UserState {
        self.users.entry(identity).or_insert_with(|| UserState {
            balance: 0,
            initialized: false,
            bets: Vec::new(),
        })
    }
    
    pub fn set_admin(&mut self, _identity: Identity, new_admin: Identity) -> Result<String, String> {
        // Admin functionality removed - this is now a no-op
        Ok(format!("Admin functionality removed - anyone can do everything now"))
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

        self.next_market_id += 1;
        let market_id = self.next_market_id;

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
        _identity: Identity,
        market_id: u64,
        outcome: bool, // true = yes won, false = no won
    ) -> Result<String, String> {
        // Anyone can resolve markets now
        
        let market = self.markets.get_mut(&market_id)
            .ok_or("Market not found")?;
        
        if market.status != MarketStatus::Open {
            return Err("Market is not open".to_string());
        }

        // Calculate payouts before changing status
        let winning_pool = if outcome { market.yes_pool } else { market.no_pool };
        let losing_pool = if outcome { market.no_pool } else { market.yes_pool };
        let total_pool = winning_pool + losing_pool;
        
        // Get winners list
        let winners: Vec<(Identity, u128)> = if outcome {
            market.yes_bettors.clone().into_iter().collect()
        } else {
            market.no_bettors.clone().into_iter().collect()
        };
        
        // Distribute winnings to all winners
        let mut total_distributed = 0u128;
        for (winner_id, stake) in winners.iter() {
            if winning_pool > 0 {
                // Calculate payout using parimutuel formula
                let payout = (*stake as f64 / winning_pool as f64 * total_pool as f64) as u128;
                
                // Add winnings to user balance
                if let Some(user) = self.users.get_mut(winner_id) {
                    user.balance += payout;
                    total_distributed += payout;
                    
                    // Mark their bet as claimed
                    if let Some(bet) = user.bets.iter_mut()
                        .find(|b| b.market_id == market_id && !b.claimed) {
                        bet.claimed = true;
                    }
                }
            }
        }

        market.status = if outcome {
            MarketStatus::ResolvedYes
        } else {
            MarketStatus::ResolvedNo
        };

        let outcome_str = if outcome { "YES" } else { "NO" };
        Ok(format!(
            "Market #{} resolved as {}. Distributed {} to {} winners", 
            market_id, outcome_str, total_distributed, winners.len()
        ))
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
    pub balance: u128,
    pub initialized: bool,
    pub bets: Vec<UserBet>,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct UserBet {
    pub market_id: u64,
    pub side: bool, // true = yes, false = no
    pub amount: u128,
    pub claimed: bool,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct Market {
    pub id: u64,
    pub creator: Identity,
    pub description: String,
    pub yes_pool: u128,
    pub no_pool: u128,
    pub yes_bettors: HashMap<Identity, u128>,
    pub no_bettors: HashMap<Identity, u128>,
    pub status: MarketStatus,
    pub created_at: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MarketStatus {
    Open,
    ResolvedYes,
    ResolvedNo,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
pub struct Contract1 {
    pub users: HashMap<Identity, UserState>,
    pub markets: HashMap<u64, Market>,
    pub next_market_id: u64,
}

impl Default for Contract1 {
    fn default() -> Self {
        Self::new()
    }
}


/// Enum representing possible calls to the contract functions
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum MarketAction {
    SetAdmin { new_admin: Identity },
    Initialize {},
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

impl Contract1 {
    pub fn as_bytes(&self) -> Result<Vec<u8>, Error> {
        borsh::to_vec(self)
    }
}

impl From<sdk::StateCommitment> for Contract1 {
    fn from(state: sdk::StateCommitment) -> Self {
        borsh::from_slice(&state.0)
            .map_err(|_| "Could not decode parimutuel market state".to_string())
            .unwrap()
    }
}
