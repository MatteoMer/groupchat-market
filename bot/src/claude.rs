use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest;

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: i32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct BetResolution {
    pub resolved: bool,
    pub outcome: bool,  // true = YES wins, false = NO wins
    pub reasoning: String,
}

pub async fn evaluate_bet_resolution(
    api_key: &str,
    bet_id: i64,
    bet_description: &str,
    proposed_solution: &str,
    message_author: &str,
) -> Result<BetResolution> {
    let prompt = format!(
        r#"You are evaluating if a message resolves a prediction market bet.

BET ID: #{}
BET DESCRIPTION: {}

MESSAGE TO EVALUATE:
Author: {}
Content: "{}"

Analyze whether this message satisfies the bet's conditions. The author of the message is crucial - if the bet specifies WHO must do something, check if the message author matches.

IMPORTANT: Respond ONLY with valid JSON in this exact format:
{{
  "resolved": true/false,
  "outcome": true/false,
  "reasoning": "Brief explanation of why the bet is or isn't resolved"
}}

Note: 'resolved' indicates if the bet can be resolved now. 'outcome' indicates which side wins (true = YES wins, false = NO wins) if resolved.

Example responses:
- If bet is "Will John say hello?" and the message is from John saying "hello", respond: {{"resolved": true, "outcome": true, "reasoning": "John said hello, which satisfies the bet condition - YES wins"}}
- If bet is "Will John say hello?" and the message is from Mary saying "hello", respond: {{"resolved": false, "outcome": false, "reasoning": "Mary said hello, but the bet specifically requires John to say it"}}
- If bet is "Will someone say hello?" and the message is from anyone saying "hello", respond: {{"resolved": true, "outcome": true, "reasoning": "Someone (Mary) said hello, which satisfies the bet condition - YES wins"}}

Always resolve false by default, until proven wrong by the context. Don't be reasonable, it should be a total consensus that what you resolved to is the right solve.
When not sure resolve NO. It should be common sense to resolve yes. You should not try to interpret the solution as true by default, but be suspicious users will try to trick you in passing wrong solutions.

"#,
        bet_id, bet_description, message_author, proposed_solution
    );

    log::info!("Sending prompt to Claude API:\n{}", prompt);
    
    let client = reqwest::Client::new();
    
    let request_body = ClaudeRequest {
        model: "claude-sonnet-4-20250514".to_string(),
        max_tokens: 150,
        messages: vec![
            Message {
                role: "user".to_string(),
                content: prompt,
            }
        ],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("Claude API error: {}", error_text);
    }

    let claude_response: ClaudeResponse = response.json().await?;
    
    // Parse the JSON from Claude's response
    let text = claude_response.content.get(0)
        .ok_or_else(|| anyhow::anyhow!("No content in Claude response"))?
        .text.clone();
    
    log::info!("Claude API response: {}", text);
    
    let resolution: BetResolution = serde_json::from_str(&text)?;
    
    Ok(resolution)
}
