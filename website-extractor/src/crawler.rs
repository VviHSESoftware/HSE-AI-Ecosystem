use crate::config::AppEnv;
use crate::kb_client::KbClient;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use tracing::{error, info, warn};
use url::Url;

pub struct Crawler {
    env: AppEnv,
    kb: KbClient,
    http: Client,
}

struct ParsedPage {
    title: String,
    text: String,
    discovered_urls: Vec<Url>,
    discovered_videos: Vec<String>,
}

impl Crawler {
    pub fn new(env: AppEnv, kb: KbClient) -> Self {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap();

        Self { env, kb, http }
    }

    pub async fn run_sync(&self) {
        info!("Starting crawl for root: {}", self.env.target_url);
        info!("Max crawl depth: {}", self.env.max_depth);

        let root_url = match Url::parse(&self.env.target_url) {
            Ok(u) => normalize_url(&u),
            Err(e) => {
                error!("Invalid target URL: {}", e);
                return;
            }
        };

        let root_ext_id = format!("page_{}", sanitize_id(root_url.path()));

        let _ = self.kb.invalidate_module(&root_ext_id, "page").await;

        let mut visited_urls = HashSet::new();

        self.crawl_page(root_url, self.env.kb_root_parent_id, 0, &mut visited_urls).await;

        info!("Crawl finished successfully! Total unique pages visited: {}", visited_urls.len());
    }

    fn crawl_page<'a>(
        &'a self,
        url: Url,
        parent_id: i32,
        depth: usize,
        visited_urls: &'a mut HashSet<Url>,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(async move {
            if !visited_urls.insert(url.clone()) {
                return;
            }

            info!("[Depth {}/{}] Crawling page: {}", depth, self.env.max_depth, url);

            let page_data = match self.fetch_and_parse(&url).await {
                Ok(data) => data,
                Err(e) => {
                    warn!("Failed to fetch page {}: {}", url, e);
                    return;
                }
            };

            let page_ext_id = format!("page_{}", sanitize_id(url.path()));

            let mod_id = match self.kb.add_module_base(
                &page_data.title,
                url.as_str(),
                &page_ext_id,
                "page",
                parent_id,
            ).await {
                Ok(id) => id,
                Err(e) => {
                    error!("Failed to register module in KB for {}: {}", url, e);
                    return;
                }
            };

            if !page_data.text.is_empty() {
                if let Err(e) = self.kb.set_text_type(mod_id, &page_data.text).await {
                    warn!("Failed to set text type for {}: {}", url, e);
                }
            }

            if !page_data.discovered_videos.is_empty() {
                info!("Found {} embedded video(s) on {}", page_data.discovered_videos.len(), url);
                for (idx, vid) in page_data.discovered_videos.iter().enumerate() {
                    let vid_ext_id = format!("{}_vid_{}", page_ext_id, idx);
                    info!("Sending video to KB: {}", vid);

                    match self.kb.add_module_base("VK Video", vid, &vid_ext_id, "video", mod_id).await {
                        Ok(v_id) => {
                            if let Err(e) = self.kb.set_video_type(v_id, vid).await {
                                error!("Failed to set video type for {}: {}", vid, e);
                            } else {
                                info!("Successfully attached video to module {}", mod_id);
                            }
                        }
                        Err(e) => {
                            error!("Failed to create video module in KB: {}", e);
                        }
                    }
                }
            }

            if depth >= self.env.max_depth {
                return;
            }

            for next_url in page_data.discovered_urls {
                let norm_url = normalize_url(&next_url);

                let is_same_host = norm_url.host_str() == Some("olymp.hse.ru");
                let is_championship_subpage = norm_url.path().starts_with("/championship");

                let is_ignored_path = norm_url.path().contains("/search")
                    || norm_url.path().contains("/tags/")
                    || norm_url.path().contains("/keywords/");

                if is_same_host && is_championship_subpage && !is_ignored_path {
                    self.crawl_page(norm_url, mod_id, depth + 1, visited_urls).await;
                }
            }
        })
    }

    async fn fetch_and_parse(&self, url: &Url) -> Result<ParsedPage, String> {
        let res = self.http.get(url.as_str()).send().await.map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("HTTP status {}", res.status()));
        }

        let html_content = res.text().await.map_err(|e| e.to_string())?;
        let document = Html::parse_document(&html_content);

        let title_selector = Selector::parse("title").unwrap();
        let title = document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| url.as_str().to_string());

        let mut discovered_urls = Vec::new();
        let mut discovered_videos = HashSet::new();

        let a_selector = Selector::parse("a[href]").unwrap();
        let iframe_selector = Selector::parse("iframe[src]").unwrap();

        for el in document.select(&a_selector) {
            if let Some(href) = el.value().attr("href") {
                let trimmed = href.trim();
                if is_video_url(trimmed) {
                    discovered_videos.insert(trimmed.to_string());
                } else if let Ok(joined) = url.join(trimmed) {
                    discovered_urls.push(joined);
                }
            }
        }

        for el in document.select(&iframe_selector) {
            if let Some(src) = el.value().attr("src") {
                let trimmed = src.trim();
                if is_video_url(trimmed) {
                    discovered_videos.insert(trimmed.to_string());
                }
            }
        }

        let content_selector = Selector::parse(".header-board--promo, .content__inner").unwrap();
        let ignore_selector = Selector::parse(
            "script, style, noscript, svg, .main-menu, .footer, .fa-footer, .gdpr_bar, .sv-control, .control-search"
        ).unwrap();

        let mut text_fragments = Vec::new();

        let content_elements: Vec<_> = document.select(&content_selector).collect();
        let target_elements = if !content_elements.is_empty() {
            content_elements
        } else {
            document.select(&Selector::parse("body").unwrap()).collect()
        };

        for target in target_elements {
            extract_clean_text(&target, &ignore_selector, &mut text_fragments);
        }

        let clean_text = text_fragments.join(" ");

        Ok(ParsedPage {
            title,
            text: clean_text,
            discovered_urls,
            discovered_videos: discovered_videos.into_iter().collect(),
        })
    }
}

fn extract_clean_text<'a>(
    element: &scraper::ElementRef<'a>,
    ignore_selector: &Selector,
    out: &mut Vec<&'a str>,
) {
    for child in element.children() {
        if let Some(child_element) = scraper::ElementRef::wrap(child) {
            if ignore_selector.matches(&child_element) {
                continue;
            }
            extract_clean_text(&child_element, ignore_selector, out);
        } else if let Some(text_node) = child.value().as_text() {
            let trimmed = text_node.text.trim();
            if !trimmed.is_empty() {
                out.push(trimmed);
            }
        }
    }
}

fn normalize_url(url: &Url) -> Url {
    let mut normalized = url.clone();
    normalized.set_fragment(None);
    normalized.set_query(None);

    let path = normalized.path().trim_end_matches('/').to_string();
    normalized.set_path(if path.is_empty() { "/" } else { &path });
    normalized
}

fn is_video_url(url_str: &str) -> bool {
    let lower = url_str.to_lowercase();
    lower.contains("vkvideo.ru")
        || lower.contains("vk.com/video")
        || lower.contains("vk.com/clip-")
        || lower.contains("video_ext.php")
}

fn sanitize_id(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}