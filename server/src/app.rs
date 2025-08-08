use std::{sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    extract::{Json, State},
    http::{HeaderMap, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use client_sdk::{
    contract_indexer::AppError,
    rest_client::{NodeApiClient, NodeApiHttpClient},
};
use contract1::{Contract1, MarketAction};

use hyle_modules::{
    bus::{BusClientReceiver, SharedMessageBus},
    module_bus_client, module_handle_messages,
    modules::{prover::AutoProverEvent, BuildApiContextInner, Module},
};
use sdk::{BlobTransaction, ContractName};
use serde::Serialize;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

pub struct AppModule {
    bus: AppModuleBusClient,
}

pub struct AppModuleCtx {
    pub api: Arc<BuildApiContextInner>,
    pub node_client: Arc<NodeApiHttpClient>,
    pub contract1_cn: ContractName,
}

module_bus_client! {
#[derive(Debug)]
pub struct AppModuleBusClient {
    receiver(AutoProverEvent<Contract1>),
}
}

impl Module for AppModule {
    type Context = Arc<AppModuleCtx>;

    async fn build(bus: SharedMessageBus, ctx: Self::Context) -> Result<Self> {
        let state = RouterCtx {
            bus: Arc::new(Mutex::new(bus.new_handle())),
            contract1_cn: ctx.contract1_cn.clone(),
            client: ctx.node_client.clone(),
        };

        // Créer un middleware CORS
        let cors = CorsLayer::new()
            .allow_origin(Any) // Permet toutes les origines (peut être restreint)
            .allow_methods(vec![Method::GET, Method::POST]) // Permet les méthodes nécessaires
            .allow_headers(Any); // Permet tous les en-têtes

        let api = Router::new()
            .route("/_health", get(health))
            .route("/api/config", get(get_config))
            // Contract1 (Market) routes
            .route("/api/market/set_admin", post(set_admin))
            .route("/api/market/initialize", post(initialize))
            .route("/api/market/create", post(create_market))
            .route("/api/market/bet", post(place_bet))
            .route("/api/market/resolve", post(resolve_market))
            .route("/api/market/claim", post(claim_winnings))
            .route("/api/market/balance", post(get_balance))
            .route("/api/market/info", post(get_market_info))
            .with_state(state)
            .layer(cors); // Appliquer le middleware CORS

        if let Ok(mut guard) = ctx.api.router.lock() {
            if let Some(router) = guard.take() {
                guard.replace(router.merge(api));
            }
        }
        let bus = AppModuleBusClient::new_from_bus(bus.new_handle()).await;

        Ok(AppModule { bus })
    }

    async fn run(&mut self) -> Result<()> {
        module_handle_messages! {
            on_bus self.bus,
        };

        Ok(())
    }
}

#[derive(Clone)]
struct RouterCtx {
    pub bus: Arc<Mutex<SharedMessageBus>>,
    pub client: Arc<NodeApiHttpClient>,
    pub contract1_cn: ContractName,
}

async fn health() -> impl IntoResponse {
    Json("OK")
}

// --------------------------------------------------------
//     Headers
// --------------------------------------------------------

const USER_HEADER: &str = "x-user";

#[derive(Debug)]
struct AuthHeaders {
    user: String,
}

impl AuthHeaders {
    fn from_headers(headers: &HeaderMap) -> Result<Self, AppError> {
        let user = headers
            .get(USER_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                AppError(
                    StatusCode::UNAUTHORIZED,
                    anyhow::anyhow!("Missing signature"),
                )
            })?;

        Ok(AuthHeaders {
            user: user.to_string(),
        })
    }
}

#[derive(Serialize)]
struct ConfigResponse {
    contract_name: String,
}


#[derive(serde::Deserialize)]
struct SetAdminRequest {
    new_admin: String,
}

#[derive(serde::Deserialize)]
struct InitializeRequest {}

#[derive(serde::Deserialize)]
struct CreateMarketRequest {
    description: String,
}

#[derive(serde::Deserialize)]
struct PlaceBetRequest {
    market_id: u64,
    side: bool,
    amount: u128,
}

#[derive(serde::Deserialize)]
struct ResolveMarketRequest {
    market_id: u64,
    outcome: bool,
}

#[derive(serde::Deserialize)]
struct ClaimWinningsRequest {
    market_id: u64,
}

#[derive(serde::Deserialize)]
struct GetBalanceRequest {}

#[derive(serde::Deserialize)]
struct GetMarketInfoRequest {
    market_id: u64,
}


// --------------------------------------------------------
//     Routes
// --------------------------------------------------------

// Contract1 (Market) routes
async fn set_admin(
    State(ctx): State<RouterCtx>,
    headers: HeaderMap,
    Json(request): Json<SetAdminRequest>
) -> Result<impl IntoResponse, AppError> {
    let auth = AuthHeaders::from_headers(&headers)?;
    let action = MarketAction::SetAdmin { new_admin: sdk::Identity(request.new_admin) };
    send_market_action(ctx, auth, action).await
}

async fn initialize(
    State(ctx): State<RouterCtx>,
    headers: HeaderMap,
    Json(_request): Json<InitializeRequest>
) -> Result<impl IntoResponse, AppError> {
    let auth = AuthHeaders::from_headers(&headers)?;
    let action = MarketAction::Initialize {};
    send_market_action(ctx, auth, action).await
}

async fn create_market(
    State(ctx): State<RouterCtx>,
    headers: HeaderMap,
    Json(request): Json<CreateMarketRequest>
) -> Result<impl IntoResponse, AppError> {
    let auth = AuthHeaders::from_headers(&headers)?;
    let action = MarketAction::CreateMarket { description: request.description };
    send_market_action(ctx, auth, action).await
}

async fn place_bet(
    State(ctx): State<RouterCtx>,
    headers: HeaderMap,
    Json(request): Json<PlaceBetRequest>
) -> Result<impl IntoResponse, AppError> {
    let auth = AuthHeaders::from_headers(&headers)?;
    let action = MarketAction::PlaceBet { 
        market_id: request.market_id,
        side: request.side,
        amount: request.amount,
    };
    send_market_action(ctx, auth, action).await
}

async fn resolve_market(
    State(ctx): State<RouterCtx>,
    headers: HeaderMap,
    Json(request): Json<ResolveMarketRequest>
) -> Result<impl IntoResponse, AppError> {
    let auth = AuthHeaders::from_headers(&headers)?;
    let action = MarketAction::ResolveMarket {
        market_id: request.market_id,
        outcome: request.outcome,
    };
    send_market_action(ctx, auth, action).await
}

async fn claim_winnings(
    State(ctx): State<RouterCtx>,
    headers: HeaderMap,
    Json(request): Json<ClaimWinningsRequest>
) -> Result<impl IntoResponse, AppError> {
    let auth = AuthHeaders::from_headers(&headers)?;
    let action = MarketAction::ClaimWinnings { market_id: request.market_id };
    send_market_action(ctx, auth, action).await
}

async fn get_balance(
    State(ctx): State<RouterCtx>,
    headers: HeaderMap,
    Json(_request): Json<GetBalanceRequest>
) -> Result<impl IntoResponse, AppError> {
    let auth = AuthHeaders::from_headers(&headers)?;
    let action = MarketAction::GetBalance;
    send_market_action(ctx, auth, action).await
}

async fn get_market_info(
    State(ctx): State<RouterCtx>,
    headers: HeaderMap,
    Json(request): Json<GetMarketInfoRequest>
) -> Result<impl IntoResponse, AppError> {
    let auth = AuthHeaders::from_headers(&headers)?;
    let action = MarketAction::GetMarketInfo { market_id: request.market_id };
    send_market_action(ctx, auth, action).await
}


async fn get_config(State(ctx): State<RouterCtx>) -> impl IntoResponse {
    Json(ConfigResponse {
        contract_name: ctx.contract1_cn.0,
    })
}

async fn send_market_action(
    ctx: RouterCtx,
    auth: AuthHeaders,
    action: MarketAction,
) -> Result<impl IntoResponse, AppError> {
    let identity = auth.user.clone();

    // Create the blob with the action
    let action_blob = action.as_blob(ctx.contract1_cn.clone());
    
    // Debug: print what we're sending
    eprintln!("Sending action: {:?}", action);
    eprintln!("Action blob contract_name: {:?}", action_blob.contract_name);
    eprintln!("Action blob data length: {}", action_blob.data.0.len());
    eprintln!("Action blob data (hex): {}", hex::encode(&action_blob.data.0));
    
    // Send just the action blob
    let blobs = vec![action_blob];

    let res = ctx
        .client
        .send_tx_blob(BlobTransaction::new(identity.clone(), blobs))
        .await;

    if let Err(ref e) = res {
        let root_cause = e.root_cause().to_string();
        return Err(AppError(
            StatusCode::BAD_REQUEST,
            anyhow::anyhow!("{}", root_cause),
        ));
    }

    let tx_hash = res.unwrap();

    let mut bus = {
        let bus = ctx.bus.lock().await;
        AppModuleBusClient::new_from_bus(bus.new_handle()).await
    };

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match bus.recv().await? {
                AutoProverEvent::<Contract1>::SuccessTx(sequenced_tx_hash, _) => {
                    if sequenced_tx_hash == tx_hash {
                        return Ok(Json(sequenced_tx_hash));
                    }
                }
                AutoProverEvent::<Contract1>::FailedTx(sequenced_tx_hash, error) => {
                    if sequenced_tx_hash == tx_hash {
                        return Err(AppError(StatusCode::BAD_REQUEST, anyhow::anyhow!(error)));
                    }
                }
            }
        }
    })
    .await?
}
