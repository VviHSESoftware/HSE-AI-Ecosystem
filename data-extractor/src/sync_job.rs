use crate::{config::AppEnv, moodle_client::MoodleClient, kb_client::KbClient};
use scraper::{Html, Selector};
use std::sync::Arc;
use tracing::{info, error, warn};

pub async fn process_webhook(env: Arc<AppEnv>, course_id: i32, update_type: String) {
    let moodle = MoodleClient::new(env.as_ref().clone());
    let kb = KbClient::new(env.as_ref().clone());

    if update_type == "tag_removed" {
        info!("Removing course {} from KB", course_id);
        let _ = kb.invalidate_module(&course_id.to_string(), "course").await;
        return;
    }

    let system_id = match kb.get_system_id().await {
        Ok(id) => id,
        Err(e) => { error!("Failed to get System ID: {}", e); return; }
    };

    if update_type == "students" {
        info!("Updating student access for course {}", course_id);
        if let Ok(Some(course_mod_id)) = kb.get_module_id(&course_id.to_string(), "course").await {
            if let Ok(users) = moodle.get_enrolled_users(course_id).await {
                let _ = kb.add_module_access(course_mod_id, users).await;
            }
        }
        return;
    }

    info!("Syncing materials for course {}", course_id);

    let _ = kb.invalidate_module(&course_id.to_string(), "course").await;

    let course = match moodle.get_course_content(course_id).await {
        Ok(c) => c,
        Err(e) => { error!("Failed to fetch course {}: {}", course_id, e); return; }
    };

    let course_mod_id = match kb.add_module_base(&course.course_name, &course.course_url, &course_id.to_string(), "course", system_id).await {
        Ok(id) => id,
        Err(e) => { error!("Failed to create course base: {}", e); return; }
    };

    if let Ok(users) = moodle.get_enrolled_users(course_id).await {
        let _ = kb.add_module_access(course_mod_id, users).await;
    }

    let office_mimes = ["pdf", "word", "powerpoint", "officedocument"];

    for section in course.sections {
        let sec_id = format!("{}_{}", course_id, section.name);
        let sec_mod_id = kb.add_module_base(&section.name, &section.url, &sec_id, "section", course_mod_id).await.unwrap_or(course_mod_id);

        let summary_text = if !section.summary.is_empty() {
            let doc = Html::parse_fragment(&section.summary);
            let text = doc.root_element().text().collect::<Vec<_>>().join(" ");
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        } else { None };

        if let Some(text) = summary_text {
            let text_id = kb.add_module_base("Section Summary", &section.url, &format!("{}_sum", sec_id), "content", sec_mod_id).await.unwrap_or(sec_mod_id);
            let _ = kb.set_text_type(text_id, &text).await;
        }

        for mod_data in section.modules {
            let act_id = format!("{}_{}", course_id, mod_data.id);
            let act_mod_id = kb.add_module_base(&mod_data.name, &mod_data.url, &act_id, "activity", sec_mod_id).await.unwrap_or(sec_mod_id);

            if !mod_data.content.is_empty() {
                let (plain_text, extracted_videos) = {
                    let iframe_sel = Selector::parse("iframe").unwrap();
                    let a_sel = Selector::parse("a").unwrap();

                    let doc = Html::parse_fragment(&mod_data.content);

                    let text = doc.root_element().text().collect::<Vec<_>>().join(" ");

                    let mut vids = Vec::new();

                    for iframe in doc.select(&iframe_sel) {
                        if let Some(src) = iframe.value().attr("src") {
                            if src.contains("vk.com/video") || src.contains("youtube.com") {
                                vids.push(src.to_string());
                            }
                        }
                    }

                    for a in doc.select(&a_sel) {
                        if let Some(href) = a.value().attr("href") {
                            if href.contains("youtube.com") || href.contains("youtu.be") || href.contains("vk.com/video") {
                                vids.push(href.to_string());
                            }
                        }
                    }

                    (text.trim().to_string(), vids)
                };


                if !plain_text.is_empty() {
                    let txt_mod_id = kb.add_module_base("Text Content", &mod_data.url, &format!("{}_txt", act_id), "content", act_mod_id).await.unwrap_or(act_mod_id);
                    let _ = kb.set_text_type(txt_mod_id, &plain_text).await;
                }

                for vid_url in extracted_videos {
                    let vid_mod_id = kb.add_module_base("Embedded Video", &vid_url, &format!("{}_vid_{}", act_id, vid_url.len()), "content", act_mod_id).await.unwrap_or(act_mod_id);
                    let _ = kb.set_video_type(vid_mod_id, &vid_url).await;
                }
            }

            for file in mod_data.files {
                let is_valid_doc = office_mimes.iter().any(|m| file.mimetype.contains(m));
                if is_valid_doc {
                    info!("Downloading file: {}", file.name);
                    if let Ok(bytes) = moodle.download_file(&file.url).await {
                        let file_ext = std::path::Path::new(&file.name).extension().and_then(|s| s.to_str()).unwrap_or("pdf");
                        let file_mod_id = kb.add_module_base(&file.name, &file.url, &format!("{}_file_{}", act_id, file.name), "content", act_mod_id).await.unwrap_or(act_mod_id);

                        if let Err(e) = kb.set_document_type(file_mod_id, &file.name, file_ext, bytes).await {
                            warn!("Failed to process document {}: {}", file.name, e);
                        }
                    }
                }
            }
        }
    }
    info!("Course {} sync completed!", course_id);
}