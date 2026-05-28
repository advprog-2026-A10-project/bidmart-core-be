use std::time::Duration;

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::{
    config::Builder as S3ConfigBuilder, presigning::PresigningConfig, Client as S3Client,
};
use uuid::Uuid;

use crate::infrastructure::config::StorageConfig;
use crate::modules::catalog::domain::errors::ListingError;

const PRESIGN_EXPIRY_SECONDS: u64 = 600;

#[derive(Clone)]
pub struct ObjectStorageService {
    client: S3Client,
    bucket: String,
    public_base_url: String,
}

#[derive(Debug, Clone)]
pub struct PresignedUpload {
    pub upload_url: String,
    pub public_url: String,
    pub object_key: String,
    pub expires_in_seconds: u64,
}

impl ObjectStorageService {
    pub async fn new(storage: &StorageConfig) -> Result<Self, ListingError> {
        let creds = Credentials::new(
            storage.access_key.clone(),
            storage.secret_key.clone(),
            None,
            None,
            "core-be-minio-env",
        );

        let base = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(creds)
            .region(aws_sdk_s3::config::Region::new(storage.region.clone()))
            .load()
            .await;

        let s3_config = S3ConfigBuilder::from(&base)
            .endpoint_url(storage.endpoint.clone())
            .force_path_style(storage.force_path_style)
            .build();

        Ok(Self {
            client: S3Client::from_conf(s3_config),
            bucket: storage.bucket.clone(),
            public_base_url: storage.public_base_url.clone(),
        })
    }

    pub async fn presign_listing_image_upload(
        &self,
        user_id: Uuid,
        file_name: &str,
        content_type: Option<&str>,
    ) -> Result<PresignedUpload, ListingError> {
        let safe_file_name = sanitize_filename(file_name)?;
        let object_key = format!("listings/{}/{}-{}", user_id, Uuid::new_v4(), safe_file_name);
        let presign_cfg = PresigningConfig::expires_in(Duration::from_secs(PRESIGN_EXPIRY_SECONDS))
            .map_err(|e| ListingError::InternalError(e.to_string()))?;

        let mut request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&object_key);

        if let Some(content_type) = content_type {
            let trimmed = content_type.trim();
            if !trimmed.is_empty() {
                request = request.content_type(trimmed);
            }
        }

        let presigned_request = request
            .presigned(presign_cfg)
            .await
            .map_err(|e| ListingError::InternalError(e.to_string()))?;

        let upload_url = presigned_request.uri().to_string();
        let public_url = format!("{}/{}", self.public_base_url, object_key);

        Ok(PresignedUpload {
            upload_url,
            public_url,
            object_key,
            expires_in_seconds: PRESIGN_EXPIRY_SECONDS,
        })
    }
}

fn sanitize_filename(file_name: &str) -> Result<String, ListingError> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err(ListingError::ValidationError(
            "file_name is required".to_string(),
        ));
    }

    let mut sanitized = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    let collapsed = sanitized.trim_matches('_').to_string();
    if collapsed.is_empty() {
        return Err(ListingError::ValidationError(
            "file_name is invalid".to_string(),
        ));
    }
    if collapsed.len() > 120 {
        return Err(ListingError::ValidationError(
            "file_name is too long".to_string(),
        ));
    }

    Ok(collapsed)
}
