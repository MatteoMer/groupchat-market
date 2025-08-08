use anyhow::{Result, anyhow};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct MarketApiClient {
    client: Client,
    base_url: String,
}


#[derive(Serialize)]
struct InitializeRequest {}

#[derive(Serialize)]
struct CreateMarketRequest {
    description: String,
}

#[derive(Serialize)]
struct PlaceBetRequest {
    market_id: u64,
    side: bool,
    amount: u128,
}

#[derive(Serialize)]
struct ResolveMarketRequest {
    market_id: u64,
    outcome: bool,
}

#[derive(Serialize)]
struct ClaimWinningsRequest {
    market_id: u64,
}

#[derive(Serialize)]
struct GetBalanceRequest {}

#[derive(Serialize)]
struct GetMarketInfoRequest {
    market_id: u64,
}

#[derive(Deserialize)]
pub struct ConfigResponse {
    pub contract_name: String,
}

impl MarketApiClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }


    pub async fn get_config(&self) -> Result<ConfigResponse> {
        let url = format!("{}/api/config", self.base_url);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to get config: {}", error_text));
        }

        let config = response.json::<ConfigResponse>().await?;
        Ok(config)
    }

    pub async fn initialize_user(&self, user_id: String, contract_name: &str) -> Result<String> {
        let url = format!("{}/api/market/initialize", self.base_url);
        let request = InitializeRequest {};

        let identity = format!("{}@{}", user_id, contract_name);
        let response = self.client
            .post(&url)
            .header("x-user", identity)
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to initialize user: {}", error_text));
        }

        let tx_hash = response.text().await?;
        Ok(tx_hash)
    }

    pub async fn create_market(&self, user_id: String, description: String, contract_name: &str) -> Result<String> {
        let url = format!("{}/api/market/create", self.base_url);
        let request = CreateMarketRequest { description };

        let identity = format!("{}@{}", user_id, contract_name);
        let response = self.client
            .post(&url)
            .header("x-user", identity)
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to create market: {}", error_text));
        }

        let tx_hash = response.text().await?;
        Ok(tx_hash)
    }

    pub async fn place_bet(&self, user_id: String, market_id: u64, side: bool, amount: u128, contract_name: &str) -> Result<String> {
        let url = format!("{}/api/market/bet", self.base_url);
        let request = PlaceBetRequest { market_id, side, amount };

        let identity = format!("{}@{}", user_id, contract_name);
        let response = self.client
            .post(&url)
            .header("x-user", identity)
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to place bet: {}", error_text));
        }

        let tx_hash = response.text().await?;
        Ok(tx_hash)
    }

    pub async fn resolve_market(&self, user_id: String, market_id: u64, outcome: bool, contract_name: &str) -> Result<String> {
        let url = format!("{}/api/market/resolve", self.base_url);
        let request = ResolveMarketRequest { market_id, outcome };

        let identity = format!("{}@{}", user_id, contract_name);
        let response = self.client
            .post(&url)
            .header("x-user", identity)
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to resolve market: {}", error_text));
        }

        let tx_hash = response.text().await?;
        Ok(tx_hash)
    }

    pub async fn claim_winnings(&self, user_id: String, market_id: u64, contract_name: &str) -> Result<String> {
        let url = format!("{}/api/market/claim", self.base_url);
        let request = ClaimWinningsRequest { market_id };

        let identity = format!("{}@{}", user_id, contract_name);
        let response = self.client
            .post(&url)
            .header("x-user", identity)
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to claim winnings: {}", error_text));
        }

        let tx_hash = response.text().await?;
        Ok(tx_hash)
    }

    pub async fn get_balance(&self, user_id: String, contract_name: &str) -> Result<String> {
        let url = format!("{}/api/market/balance", self.base_url);
        let request = GetBalanceRequest {};

        let identity = format!("{}@{}", user_id, contract_name);
        let response = self.client
            .post(&url)
            .header("x-user", identity)
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to get balance: {}", error_text));
        }

        let balance = response.text().await?;
        Ok(balance)
    }

    pub async fn get_market_info(&self, user_id: String, market_id: u64, contract_name: &str) -> Result<String> {
        let url = format!("{}/api/market/info", self.base_url);
        let request = GetMarketInfoRequest { market_id };

        let identity = format!("{}@{}", user_id, contract_name);
        let response = self.client
            .post(&url)
            .header("x-user", identity)
            .json(&request)
            .send()
            .await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow!("Failed to get market info: {}", error_text));
        }

        let info = response.text().await?;
        Ok(info)
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/_health", self.base_url);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        Ok(response.status() == StatusCode::OK)
    }
}