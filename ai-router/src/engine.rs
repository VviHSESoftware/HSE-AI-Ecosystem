use crate::{schemas::*, state::RouterState};
use serde_json::{json};
use tracing::{info, error};

use regex::Regex;
use std::sync::LazyLock;

static TIMECODE_URL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"https?://[^\s\)\]"'<>]+&t=[^\s\)\]"'<>]+"#).unwrap()
});

pub async fn process_pipeline(state: &RouterState, req: RouterRequest) -> Result<RouterResponse, String> {
    let start = std::time::Instant::now();
    info!("Request started");
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
   - "keywords" Leave empty.
   {{"query_type": "semantic_query", "module_ids": [123, 456], "semantic_query": "question?", "keywords": []}}

2. Timed Video Query (Specific timeframe in a video)
   {{"query_type": "timed_video_query", "module_id": 123, "second_start": 0, "second_end": 60}}

3. Document Page Query (Specific page in a file)
   {{"query_type": "document_page_query", "module_id": 123, "page": 5}}

4. Full Content Query (Read entire module/document)
   - Use ONLY for modules with type "Text" and "File"
   - DO NOT USE on modules with type "Base"
   {{"query_type": "full_content_query", "module_ids": [123]}}

If you are not sure - select the semantic query on all accessible Structure roots.
"#);

    let mut router_msgs = vec![ChatMessage { role: "system".into(), content: router_system_prompt }];
    router_msgs.extend(req.messages.clone());

    let router_raw_res = state.call_llm(router_msgs, &state.env.router_model_mode, 0.1).await?;
    info!("LLM Call 1 ended {}", start.elapsed().as_secs_f64());
    let last_msg = &req.messages.last().map(|m| m.content.as_str()).unwrap_or("");
    let decision = state.extract_json_from_llm(&router_raw_res).unwrap_or(json!({"query_type": "semantic_query", "module_ids": [], "semantic_query": last_msg, "keywords": []}));
    info!("Router raw response: {}", router_raw_res);
    let query_type = decision["query_type"].as_str().unwrap_or("nodata");

    info!("Router decided tool: {}", query_type);

    let mut retrieved_chunks = vec![];
    let module_used = query_type.to_string();

    // if query_type == "direct_answer" {
    //     let ans = decision["direct_answer"].as_str().unwrap_or("Не удалось найти ответ.");
    //     info!("Request ended with a direct answer {}", start.elapsed().as_secs_f64());
    //     return Ok(RouterResponse { content: ans.to_string(), sources: vec![], module_used });
    // }

    if query_type != "nodata" {
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

        info!("KB Query ended {}", start.elapsed().as_secs_f64());
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
        "You MUST integrate a reference and link to the source chunk in your TEXT. Video/vk/youtube links MUST include a timecode (at the end of the link via &t=XXXs). If you are not given any link - DO NOT GUESS the link."
    } else {
        "Do not use any URLs in your response. Just mention the source you used for your response, for example 'В ХХХ говорится, что ...'."
    };

    let format_rule = if req.use_markdown {
        "FORMATING MODE: FORMATED. Enclose text into _ for italic, into * for bold. Use • for bullet points. Use [Text](url) for links. For Text you can use Context description from the Chunks."
    } else {
        "FORMATING MODE: RAW. Do not use any formating."
    };

    let voice_rule = if req.voice_mode {
        "MODE: VOICE ASSISTANT. NEVER use math symbols (*, +, =, (, )). Spell out abbreviations as they sound."
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
    info!("LLM Call 2 ended {}", start.elapsed().as_secs_f64());

    let raw_content  = final_json["response"].as_str().unwrap_or("").to_string();

    let content = TIMECODE_URL_RE.replace_all(raw_content.as_str(), |caps: &regex::Captures| {
        let url = &caps[0];
        let lower = url.to_lowercase();

        let is_vk_or_yt = lower.contains("vk.com")
            || lower.contains("vkvideo")
            || lower.contains("vk.ru")
            || lower.contains("youtube.com")
            || lower.contains("youtu.be");

        if is_vk_or_yt {
            url.to_string()
        } else {
            url.replacen("&t=", "#t=", 1)
        }
    }).into_owned();

    let urls: Vec<String> = final_json["urls"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str())
        .map(|u| {
            let lower = u.to_lowercase();
            let is_vk_or_yt = lower.contains("vk.com")
                || lower.contains("vkvideo")
                || lower.contains("vk.ru")
                || lower.contains("youtube.com")
                || lower.contains("youtu.be");

            if is_vk_or_yt {
                u.to_string()
            } else {
                u.replacen("&t=", "#t=", 1)
            }
        })
        .collect();

    let sources = urls
        .into_iter()
        .map(|u| SourceMaterial {
            title: "Материал источника".into(),
            url: u,
            r#type: "link".into(),
        })
        .collect();

    info!("Request ended fully {}", start.elapsed().as_secs_f64());
    Ok(RouterResponse { content, sources, module_used })
}