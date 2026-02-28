use super::config::LlmConfig;
use agentropic_cognition::BeliefBase;
use tracing::warn;

/// Call an LLM with the question and belief context
pub async fn ask_llm(question: &str, config: &LlmConfig, beliefs: &BeliefBase) -> Option<String> {
    let client = reqwest::Client::new();

    // Build context from existing beliefs
    let mut context = String::from("Here are verified facts about Agentropic:\n");
    for (i, belief) in beliefs.all().enumerate() {
        if i >= 10 { break; }
        context.push_str(&format!("- {}\n", belief.value()));
    }

    let system_prompt = format!(
        "You are a knowledgeable assistant for Agentropic, a multi-agent framework built in Rust.\n\
         {}\n\
         Use ONLY the facts above to answer. If the facts don't cover the question, say so honestly.\n\
         Always spell the name correctly: Agentropic.\n\
         Answer concisely in 1-3 sentences.",
        context
    );

    match config.llm_provider.as_str() {
        "ollama" => call_ollama(question, config, &system_prompt, &client).await,
        "claude" => call_claude(question, config, &system_prompt, &client).await,
        other => {
            warn!("Unknown LLM provider: {}", other);
            None
        }
    }
}

async fn call_ollama(
    question: &str,
    config: &LlmConfig,
    system_prompt: &str,
    client: &reqwest::Client,
) -> Option<String> {
    let body = serde_json::json!({
        "model": config.llm_model,
        "prompt": format!("{}\n\nQuestion: {}\nAnswer:", system_prompt, question),
        "stream": false
    });

    match client.post(&config.ollama_url).json(&body).send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json["response"].as_str().map(|s| s.trim().to_string())
            } else {
                None
            }
        }
        Err(e) => {
            warn!("Ollama error: {}", e);
            None
        }
    }
}

async fn call_claude(
    question: &str,
    config: &LlmConfig,
    system_prompt: &str,
    client: &reqwest::Client,
) -> Option<String> {
    let body = serde_json::json!({
        "model": config.llm_model,
        "max_tokens": 200,
        "system": system_prompt,
        "messages": [{
            "role": "user",
            "content": question
        }]
    });

    match client
        .post(&config.claude_url)
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json["content"][0]["text"].as_str().map(|s| s.trim().to_string())
            } else {
                None
            }
        }
        Err(e) => {
            warn!("Claude error: {}", e);
            None
        }
    }
}
