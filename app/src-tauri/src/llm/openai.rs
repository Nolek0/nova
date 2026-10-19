//! Client de flux au format OpenAI « chat completions », parlé par llama-server et Ollama.
//! Les adresses sont toujours locales (127.0.0.1).

use super::*;
use futures_util::StreamExt;

pub async fn stream_chat(
    base_url: &str,
    model: &str,
    req: &GenRequest,
    think: bool,
    cancel: &Cancel,
    mut on_chunk: impl FnMut(Chunk),
) -> Result<(), GenError> {
    let mut messages = vec![serde_json::json!({"role": "system", "content": req.instructions})];
    for t in &req.history {
        messages.push(serde_json::json!({"role": t.role, "content": t.text}));
    }
    if req.images.is_empty() {
        messages.push(serde_json::json!({"role": "user", "content": req.prompt}));
    } else {
        let mut parts = vec![serde_json::json!({"type": "text", "text": req.prompt})];
        for img in &req.images {
            if let Some(url) = image_data_url(img) {
                parts.push(serde_json::json!({"type": "image_url", "image_url": {"url": url}}));
            }
        }
        messages.push(serde_json::json!({"role": "user", "content": parts}));
    }
    let mut body = serde_json::json!({"model": model, "messages": messages, "stream": true, "cache_prompt": true});
    // Réponse rapide : la phase de réflexion des modèles qui en ont une (Gemma 4, Qwen3) est coupée.
    // Un « bonjour » passe ainsi d'environ 14 s à moins d'une seconde.
    body["chat_template_kwargs"] = serde_json::json!({ "enable_thinking": think });
    body["reasoning_effort"] = serde_json::json!(if think { "high" } else { "low" });
    let client = reqwest::Client::builder().no_proxy().build().map_err(|e| GenError::Other(e.to_string()))?;
    let resp = client
        .post(format!("{base_url}/v1/chat/completions"))
        .json(&body)
        .send()
        .await
        .map_err(|e| GenError::Unavailable(format!("Moteur local injoignable : {e}")))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let detail = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v["error"]["message"].as_str().map(String::from))
            .unwrap_or(text);
        if detail.contains("context") && detail.contains("exceed") {
            return Err(GenError::Other("La conversation dépasse la mémoire de travail du modèle. Commence une nouvelle conversation.".into()));
        }
        return Err(GenError::Other(format!("Erreur du moteur ({status}) : {}", detail.chars().take(300).collect::<String>())));
    }
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    while let Some(item) = stream.next().await {
        if cancel.is_cancelled() {
            return Err(GenError::Cancelled);
        }
        let bytes = item.map_err(|e| GenError::Other(format!("Flux interrompu : {e}")))?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer.drain(..=pos);
            let Some(data) = line.strip_prefix("data:") else { continue };
            let data = data.trim();
            if data == "[DONE]" {
                return Ok(());
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(data) else { continue };
            if let Some(err) = v["error"]["message"].as_str() {
                return Err(GenError::Other(err.to_string()));
            }
            let delta = &v["choices"][0]["delta"];
            if let Some(r) = delta["reasoning_content"].as_str().or_else(|| delta["reasoning"].as_str()) {
                if !r.is_empty() {
                    on_chunk(Chunk::Reasoning(r.to_string()));
                }
            }
            if let Some(t) = delta["content"].as_str() {
                if !t.is_empty() {
                    on_chunk(Chunk::Text(t.to_string()));
                }
            }
        }
    }
    Ok(())
}

/// Modèles installés dans Ollama, s'il tourne.
pub async fn ollama_models() -> Vec<ModelInfo> {
    let client = match reqwest::Client::builder().no_proxy().timeout(std::time::Duration::from_secs(2)).build() {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let Ok(resp) = client.get("http://127.0.0.1:11434/api/tags").send().await else { return vec![] };
    let Ok(v) = resp.json::<serde_json::Value>().await else { return vec![] };
    let mut out = Vec::new();
    for m in v["models"].as_array().cloned().unwrap_or_default() {
        let name = m["name"].as_str().unwrap_or_default().to_string();
        let caps = client
            .post("http://127.0.0.1:11434/api/show")
            .json(&serde_json::json!({"model": name}))
            .send()
            .await
            .ok();
        let vision = match caps {
            Some(r) => r.json::<serde_json::Value>().await.ok().and_then(|j| j["capabilities"].as_array().cloned()).unwrap_or_default().iter().any(|c| c == "vision"),
            None => false,
        };
        out.push(ModelInfo { id: format!("ollama:{name}"), provider: "ollama".into(), name, vision, local: true, size_bytes: m["size"].as_u64() });
    }
    out
}
