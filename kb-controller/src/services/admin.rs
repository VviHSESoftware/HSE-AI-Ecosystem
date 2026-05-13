use sha2::{Digest, Sha256};
use kb_common::repository::KbRepository;
use crate::schemas::CreateSystemRes;
use common_infra::AppError;

pub struct AdminService {
    pub repo: KbRepository,
}

impl AdminService {
    pub fn new(repo: KbRepository) -> Self {
        Self { repo }
    }

    pub async fn create_system(&self, name: &str) -> Result<CreateSystemRes, AppError> {
        let raw_token = uuid::Uuid::new_v4().to_string();

        let mut hasher = Sha256::new();
        hasher.update(raw_token.as_bytes());
        let hash = hex::encode(hasher.finalize());

        let system_id = self.repo.create_system(name, &hash)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(CreateSystemRes {
            system_id,
            system_token: raw_token,
            note: "Save this token. It won't be shown again.".to_string(),
        })
    }
}