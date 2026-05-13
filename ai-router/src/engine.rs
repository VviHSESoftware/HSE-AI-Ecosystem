use crate::{schemas::*, state::RouterState};
use serde_json::{json};
use tracing::{info, error};

pub async fn process_pipeline(state: &RouterState, req: RouterRequest) -> Result<RouterResponse, String> {
    let email = req.user_email.clone().unwrap_or_else(|| "anonymous".to_string());

    let structure = state.get_kb_structure(&email).await.unwrap_or(json!({"modules": []}));

    let router_system_prompt = format!(r#"
You are a routing agent for a university Knowledge Base.
Analyze the user's conversation and choose the exact tool to fetch data.

Available Structure for this user:
{structure}

You MUST return ONLY a JSON object choosing ONE of the following formats:

1. Semantic Query (General search by meaning)
   - "semantic_query" MUST be a question ending with '?'.
   - "keywords" ONLY if 100% sure the exact word is present.
   {{"query_type": "semantic_query", "module_ids": [123, 456], "semantic_query": "question?", "keywords": []}}

2. Timed Video Query (Specific timeframe in a video)
   {{"query_type": "timed_video_query", "module_id": 123, "second_start": 0, "second_end": 60}}

3. Document Page Query (Specific page in a file)
   {{"query_type": "document_page_query", "module_id": 123, "page": 5}}

4. Full Content Query (Read entire module/document)
   - works ONLY for "text" and "file" modules
   - not recomended for modules having submodules
   {{"query_type": "full_content_query", "module_ids": [123]}}

5. Direct Answer (ONLY IF a page or timecode requested and it is out of bounds)
   {{"query_type": "direct_answer", "language": "ru", "direct_answer": "Извините, границы некорректны..."}}
"#);

    info!(router_system_prompt);

    let mut router_msgs = vec![ChatMessage { role: "system".into(), content: router_system_prompt }];
    router_msgs.extend(req.messages.clone());

    let router_raw_res = state.call_llm(router_msgs, &state.env.router_model_mode, 0.1).await?;
    let decision = state.extract_json_from_llm(&router_raw_res)?;
    info!("Router raw response: {}", router_raw_res);
    let query_type = decision["query_type"].as_str().unwrap_or("direct_answer");

    info!("Router decided tool: {}", query_type);

    let mut retrieved_chunks = vec![];
    let module_used = query_type.to_string();

    if query_type == "direct_answer" {
        let ans = decision["direct_answer"].as_str().unwrap_or("Не удалось найти ответ.");
        return Ok(RouterResponse { content: ans.to_string(), sources: vec![], module_used });
    }

    let kb_endpoint = match query_type {
        "semantic_query" => "semanticQuery",
        "timed_video_query" => "timedVideoQuery",
        "document_page_query" => "documentPageQuery",
        "full_content_query" => "fullContentQuery",
        _ => "semanticQuery"
    };

    let mut kb_payload = decision.clone();
    if let Some(obj) = kb_payload.as_object_mut() { obj.insert("email".into(), json!(email)); }

    match state.query_kb(kb_endpoint, kb_payload).await {
        Ok(chunks) => retrieved_chunks = chunks,
        Err(e) => {
            error!("KB Query failed: {}", e);
        }
    }

    let mut context_parts = vec![];
    for (i, chunk) in retrieved_chunks.iter().enumerate() {
        context_parts.push(format!("--- Chunk {} ---\nContext: {}\nText: {}\nURL: {}", i+1, chunk.desc, chunk.text, chunk.url));
    }

    let context_compiled = if context_parts.is_empty() {
        "NO RELEVANT MATERIALS FOUND IN THE KNOWLEDGE BASE.".to_string()
    } else {
        context_parts.join("\n\n")
    };

    let url_rule = if req.integrate_links_in_text {
        "You MUST integrate a reference and link to the source chunk in your text. Video/vk/youtube links MUST include a timecode (at the end of the link via &t=XXXs)."
    } else {
        "Do not use any URLs in your response."
    };

    let format_rule = if req.use_markdown {
        "FORMATING MODE: MARKDOWN. Use markdown formating (*, **). Use [Text](url) for links. For Text you can use Context description."
    } else {
        "FORMATING MODE: RAW. Do not use any formating."
    };

    let voice_rule = if req.voice_mode {
        "MODE: ALICE (Voice-assistant). NEVER use math symbols (*, +, =, (, )). Transform notation into natural Russian speech (e.g. 'ноль целых пять сотых'). Spell out abbreviations as they sound."
    } else {
        "MODE: TEXT. Use Unicode symbols for formulas."
    };

    let final_system_prompt = format!(r#"You are the "HSE AI Tutor," an academic assistant.

STRICT OPERATING RULES:
1. LANGUAGE: You MUST answer ONLY in Russian.
2. SOURCE MATERIAL: Check the provided context. If info is missing, state 'В моей базе знаний нет информации об ХХХ, но...'.
3. FORMATTING:
   {voice_rule}
   {url_rule}
   {format_rule}
4. TONE: Professional, use "Вы". Max 3-4 sentences.
5. Never mention Chunk numbers in the sources.

PROVIDED CONTEXT (RAG RESULTS):
{context_compiled}

You MUST return ONLY a JSON object:
{{"response": "Ваш ответ здесь...", "urls": ["https://vk.com/....&t=123", "https://lms.test.com/..."]}}
"#);

    let mut final_msgs = vec![ChatMessage { role: "system".into(), content: final_system_prompt }];
    final_msgs.extend(req.messages);

    let final_raw_res = state.call_llm(final_msgs, &state.env.generation_model_mode, 0.2).await?;
    let final_json = state.extract_json_from_llm(&final_raw_res).unwrap_or(json!({ "response": final_raw_res, "urls": [] }));

    let content = final_json["response"].as_str().unwrap_or("").to_string();
    let urls: Vec<String> = final_json["urls"].as_array().unwrap_or(&vec![]).iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect();

    let sources = urls.into_iter().map(|u| SourceMaterial { title: "Материал источника".into(), url: u, r#type: "link".into() }).collect();

    Ok(RouterResponse { content, sources, module_used })
}