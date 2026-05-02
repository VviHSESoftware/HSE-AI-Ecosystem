use std::fs;
use std::sync::Arc;
use futures::future::join_all;
use regex::Regex;
use tempfile::Builder;
use tokio::process::Command;
use crate::config::AppEnv;
use ai_gateway_client::{AiGateway, VLMRequest};
use tokio::sync::Semaphore;
use tracing::{error, info};
use tracing::log::warn;
use url::Url;
use common_infra::AppError;
use doc_processor_client::{DocumentPagePart, ParseDocumentResponse, ParseVideoResponse, PartitionPart};

pub struct ParsingService {
    env: AppEnv,
    client: reqwest::Client,
    ai_gateway: Arc<dyn AiGateway>,
    semaphore: Semaphore,
}

impl ParsingService {
    pub fn new(env: AppEnv, ai_gateway: Arc<dyn AiGateway>) -> Self {
        Self {
            env,
            client: common_infra::get_common_client(),
            ai_gateway,
            semaphore: Semaphore::new(5),
        }
    }

    pub async fn parse_video(&self, raw_url: &str) -> Result<ParseVideoResponse, AppError> {
        let url = self.normalize_vk_url(raw_url).await;
        info!("Processing video URL: {}", url);

        let temp_dir = Builder::new().prefix("vid_proc_").tempdir().map_err(|e| e.to_string())?;
        let output_path = temp_dir.path().join("audio.%(ext)s");
        let expected_mp3 = temp_dir.path().join("audio.mp3");

        let output = Command::new("yt-dlp")
            .args([
                "--format", "bestaudio/best",
                "-o", output_path.to_str().ok_or("Path contains invalid UTF-8")?,
                "--extract-audio",
                "--audio-format", "mp3",
                "--audio-quality", "32K",
                "--postprocessor-args", "ffmpeg:-ac 1 -ar 16000",
                "--quiet", "--no-warnings",
                "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36",
                "--referer", "https://vk.com/",
                &url
            ])
            .output().await.map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;

        if !output.status.success() {
            let err_msg = String::from_utf8_lossy(&output.stderr);
            error!("yt-dlp error: {}", err_msg);
            return Err("Failed to download video".into());
        }

        let audio_bytes = fs::read(&expected_mp3).map_err(|e| format!("Failed to read audio file: {}", e))?;

        let data = self.ai_gateway
            .transcribe_audio("audio.mp3".to_string(), audio_bytes)
            .await
            .map_err(|e| e.to_string())?;

        let text = data["text"].as_str().unwrap_or_default().to_string();
        let mut parts = Vec::new();
        if let Some(segments) = data["segments"].as_array() {
            for s in segments {
                parts.push(PartitionPart {
                    text: s["text"].as_str().unwrap_or_default().to_string(),
                    start: s["start"].as_f64().unwrap_or(0.0),
                    end: s["end"].as_f64().unwrap_or(0.0),
                });
            }
        }

        Ok(ParseVideoResponse { text, parts })
    }

    #[tracing::instrument(skip(self, file_bytes))]
    pub async fn process_document(&self, doc_type: &str, file_bytes: Vec<u8>) -> Result<ParseDocumentResponse, AppError> {
        let _permit = self.semaphore.acquire().await.map_err(|_| AppError::Internal("Semaphore error".into()))?;

        let temp_dir = Builder::new().prefix("doc_").tempdir().map_err(|e| e.to_string())?;

        let image_paths = self.convert_to_images(temp_dir.path(), doc_type, file_bytes).await?;

        let (full_text, all_parts) = self.ocr_pages_via_vlm(image_paths).await?;

        Ok(ParseDocumentResponse {
            text: full_text.trim().to_string(),
            page_count: all_parts.len(),
            parts: all_parts,
        })
    }
}

impl ParsingService {
    #[tracing::instrument(skip(self))]
    async fn convert_to_images(&self, temp_dir: &std::path::Path, doc_type: &str, bytes: Vec<u8>) -> Result<Vec<std::path::PathBuf>, AppError> {
        let input_path = temp_dir.join(format!("in.{}", doc_type));
        fs::write(&input_path, &bytes).map_err(|e| format!("Failed to write file {:?}: {}", input_path, e))?;

        let is_office = ["docx", "pptx", "doc", "ppt"].contains(&doc_type);
        let is_pdf = doc_type == "pdf";
        let is_image = ["png", "jpg", "jpeg"].contains(&doc_type);

        if is_office {
            let status = Command::new("soffice")
                .args([
                    "--headless", "--convert-to", "pdf",
                    "--outdir", temp_dir.to_str().ok_or("Path contains invalid UTF-8")?,
                    input_path.to_str().ok_or("Path contains invalid UTF-8")?
                ])
                .status().await.map_err(|e| format!("LibreOffice error: {}", e))?;

            if !status.success() { return Err("Failed to convert Office to PDF".into()); }
        }

        if is_office || is_pdf {
            let target_pdf = if is_office { temp_dir.join("in.pdf") } else { input_path };

            let status = Command::new("pdftoppm")
                .args([
                    "-png",
                    target_pdf.to_str().ok_or("Path contains invalid UTF-8")?,
                    temp_dir.join("p").to_str().ok_or("Path contains invalid UTF-8")?
                ])
                .status().await.map_err(|e| format!("pdftoppm error: {}", e))?;

            if !status.success() { return Err("Failed to render PDF to images".into()); }
        } else if is_image {
            let final_path = temp_dir.join("p-1.png");

            fs::rename(&input_path, &final_path).map_err(|e| e.to_string())?;
        }

        let mut image_paths = fs::read_dir(temp_dir).map_err(|e| format!("Failed to read directory {:?}: {}", temp_dir, e))?
            .filter_map(|e| e.ok()).map(|e| e.path())
            .filter(|p| {
                let is_png = p.extension()
                    .map_or(false, |ext| ext == "png");

                let starts_with_p = p.file_name()
                    .map(|name| name.to_string_lossy().starts_with('p'))
                    .unwrap_or(false);

                is_png && starts_with_p
            })
            .collect::<Vec<_>>();
        image_paths.sort();

        Ok(image_paths)
    }

    async fn ocr_pages_via_vlm(&self, image_paths: Vec<std::path::PathBuf>) -> Result<(String, Vec<DocumentPagePart>), AppError> {
        let mut all_parts = Vec::new();
        let mut full_text = String::new();
        let mut current_page_offset = 1;

        for batch in image_paths.chunks(self.env.vlm_batch_size) {
            let mut upload_tasks = vec![];
            let mut batch_files = vec![];

            for path in batch {
                let uuid = uuid::Uuid::new_v4().to_string();
                let dufs_url = format!("{}/{}.png", self.env.dufs_url.trim_end_matches('/'), uuid);
                batch_files.push(dufs_url.clone());

                let img_data = fs::read(path).map_err(|e| format!("Failed to read image {:?}: {}", path, e))?;
                let client = self.client.clone();
                let user = self.env.dufs_user.clone();
                let pass = self.env.dufs_pass.clone();

                upload_tasks.push(tokio::spawn(async move {
                    client.put(&dufs_url)
                        .basic_auth(user, Some(pass))
                        .body(img_data)
                        .send().await
                }));
            }

            join_all(upload_tasks).await;

            let prompt = format!(
                "You are provided with {} images of document pages. \
                 Extract text from each page into Markdown. \
                 If the page contains images, replace them with [[IMAGE: DESCRIPTION]], \
                 where DESCRIPTION is a detailed description of an image. \
                 CRITICAL: Wrap EACH page content in <PAGE_START> and <PAGE_END> tags. Double check no tags are missing!",
                batch.len()
            );

            info!("STARTED VLM REQUEST WITH {} PAGES", batch.len());

            let vlm_req = VLMRequest {
                text: prompt,
                image_urls: batch_files.clone(),
                temperature: Some(0.2),
                max_tokens: None,
            };

            let resp = self.ai_gateway
                .vlm_analyze(vlm_req)
                .await
                .map_err(|e| format!("VLM Gateway error: {}", e))?;

            let combined_content = &resp.content;

            let page_contents = self.extract_all_pages(combined_content);

            if page_contents.len() != batch.len() {
                error!("Incomplete VLM response. Sent {}, got {}. Content: {}", batch.len(), page_contents.len(), combined_content);
                return Err(format!("VLM integrity failed: expected {} pages, got {}", batch.len(), page_contents.len()).into());
            }

            for (i, content) in page_contents.into_iter().enumerate() {
                let page_num = current_page_offset + i;
                all_parts.push(DocumentPagePart {
                    text: content.clone(),
                    page_number: page_num,
                });
                full_text.push_str(&content);
                full_text.push_str("\n\n");
            }

            current_page_offset += batch.len();

            let mut delete_tasks = vec![];
            for url in batch_files {
                let client = self.client.clone();
                let user = self.env.dufs_user.clone();
                let pass = self.env.dufs_pass.clone();
                delete_tasks.push(tokio::spawn(async move {
                    let _ = client.delete(&url).basic_auth(user, Some(pass)).send().await;
                }));
            }
            join_all(delete_tasks).await;
        }

        Ok((full_text.trim().to_string(), all_parts))
    }

    async fn normalize_vk_url(&self, url: &str) -> String {
        if !url.contains("video_ext.php") { return url.to_string(); }

        let parsed = match Url::parse(url) {
            Ok(p) => p,
            Err(_) => return url.to_string(),
        };

        let oid = parsed.query_pairs().find(|(k, _)| k == "oid").map(|(_, v)| v.into_owned());
        let vid = parsed.query_pairs().find(|(k, _)| k == "id").map(|(_, v)| v.into_owned());

        if let (Some(oid), Some(vid)) = (oid, vid) {
            if let Ok(resp) = self.client.get(url)
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36")
                .send().await
            {
                if let Ok(html) = resp.text().await {
                    let re_list_json = Regex::new(r#""list"\s*:\s*"([a-zA-Z0-9_-]+)""#).unwrap();
                    let re_list_url = Regex::new(r#"[?&]list=([a-zA-Z0-9_-]+)"#).unwrap();

                    let list_token = re_list_json.captures(&html)
                        .or_else(|| re_list_url.captures(&html))
                        .and_then(|cap| cap.get(1).map(|m| m.as_str().to_string()));

                    if let Some(token) = list_token {
                        return format!("https://vk.com/video{}_{}?list={}", oid, vid, token);
                    }
                }
            }
        }
        url.to_string()
    }

    fn extract_all_pages(&self, text: &str) -> Vec<String> {
        let mut results = Vec::new();
        let start_tag = "<PAGE_START>";
        let end_tags = ["</PAGE_END>", "<PAGE_END>"];

        let mut current_pos = 0;

        while let Some(start_idx) = text[current_pos..].find(start_tag) {
            let absolute_start = current_pos + start_idx + start_tag.len();

            let mut found_end = None;
            for tag in end_tags {
                if let Some(end_idx) = text[absolute_start..].find(tag) {
                    found_end = Some((end_idx, tag.len()));
                    break;
                }
            }

            if let Some((end_idx, tag_len)) = found_end {
                let absolute_end = absolute_start + end_idx;
                let page_content = text[absolute_start..absolute_end].trim().to_string();

                if !page_content.is_empty() {
                    results.push(page_content);
                }

                current_pos = absolute_end + tag_len;
            } else {
                warn!("Missing closing tag for page starting at {}", absolute_start);
                break;
            }
        }
        results
    }
}
