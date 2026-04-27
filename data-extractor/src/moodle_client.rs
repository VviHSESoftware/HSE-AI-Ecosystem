use serde::Deserialize;
use reqwest::Client;
use crate::config::AppEnv;

#[derive(Debug, Deserialize)]
pub struct MoodleFile {
    pub name: String,
    pub url: String,
    pub mimetype: String,
}

#[derive(Debug, Deserialize)]
pub struct MoodleModule {
    pub id: i32,
    pub r#type: String,
    pub name: String,
    pub url: String,
    pub content: String,
    pub files: Vec<MoodleFile>,
}

#[derive(Debug, Deserialize)]
pub struct MoodleSection {
    pub name: String,
    pub url: String,
    pub summary: String,
    pub modules: Vec<MoodleModule>,
}

#[derive(Debug, Deserialize)]
pub struct MoodleCourse {
    pub course_id: i32,
    pub course_name: String,
    pub course_url: String,
    pub sections: Vec<MoodleSection>,
}

#[derive(Debug, Deserialize)]
pub struct MoodleUser {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct MoodleUsersResponse {
    pub users: Vec<MoodleUser>,
}

pub struct MoodleClient {
    env: AppEnv,
    client: Client,
}

impl MoodleClient {
    pub fn new(env: AppEnv) -> Self {
        Self { env, client: Client::new() }
    }

    pub async fn get_course_content(&self, course_id: i32) -> Result<MoodleCourse, String> {
        let url = format!("{}/webservice/rest/server.php", self.env.moodle_url);
        let res = self.client.get(&url)
            .query(&[
                ("wstoken", &self.env.moodle_wstoken),
                ("wsfunction", &"local_data_extractor_get_course_content_batch".to_string()),
                ("moodlewsrestformat", &"json".to_string()),
                ("courseids[0]", &course_id.to_string())
            ])
            .send().await.map_err(|e| e.to_string())?;

        let text = res.text().await.map_err(|e| e.to_string())?;

        if text.starts_with('{') && text.contains("\"exception\"") {
            return Err(format!("Moodle API Exception: {}", text));
        }
        
        let mut data: Vec<MoodleCourse> = serde_json::from_str(&text).map_err(|e| {
            format!("JSON Decode Error: {}. Raw body: {}", e, text)
        })?;

        data.pop().ok_or_else(|| "Course list is empty".into())
    }

    pub async fn get_enrolled_users(&self, course_id: i32) -> Result<Vec<String>, String> {
        let url = format!("{}/webservice/rest/server.php", self.env.moodle_url);
        let res = self.client.get(&url)
            .query(&[
                ("wstoken", &self.env.moodle_wstoken),
                ("wsfunction", &"local_data_extractor_get_course_users_batch".to_string()),
                ("moodlewsrestformat", &"json".to_string()),
                ("courseids[0]", &course_id.to_string())
            ])
            .send().await.map_err(|e| e.to_string())?;

        let mut data: Vec<MoodleUsersResponse> = res.json().await.map_err(|e| e.to_string())?;
        let users = data.pop().map(|d| d.users.into_iter().map(|u| u.email).collect()).unwrap_or_default();
        Ok(users)
    }

    pub async fn download_file(&self, file_url: &str) -> Result<Vec<u8>, String> {
        let mut download_url = file_url.to_string();
        if download_url.contains("pluginfile.php") && !download_url.contains("webservice/pluginfile.php") {
            download_url = download_url.replace("pluginfile.php", "webservice/pluginfile.php");
        }

        let mut request = self.client.get(&download_url);
        if download_url.contains("pluginfile") {
            request = request.query(&[("token", &self.env.moodle_wstoken)]);
        }

        let res = request.send().await.map_err(|e| e.to_string())?;
        if !res.status().is_success() { return Err(format!("Failed to download file: HTTP {}", res.status())); }

        Ok(res.bytes().await.map_err(|e| e.to_string())?.to_vec())
    }
}