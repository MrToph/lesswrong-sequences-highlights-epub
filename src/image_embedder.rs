use anyhow::{Context, Error, Result};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use url::Url;

use crate::cache::Cache;
use lesswrong_api::Post;

/// Result of attempting to embed an image
#[derive(Debug)]
pub enum EmbeddingResult {
    /// HTML text to be used instead of the image (e.g., <a/> links)
    Text(String),
    /// Embedded image data
    Image(ImageEmbedding),
}

/// Embedded image data
#[derive(Debug, Serialize, Deserialize)]
pub struct ImageEmbedding {
    /// Unique identifier for the image
    pub id: String,
    /// Original URL of the image
    pub old_url: String,
    /// Raw image bytes
    pub image_bytes: Vec<u8>,
}

/// Component to handle fetching and embedding images
pub struct ImageEmbedder {
    client: ReqwestClient,
    cache_images: Cache<Vec<u8>>,
}

impl Default for ImageEmbedder {
    fn default() -> Self {
        Self {
            client: ReqwestClient::new(),
            cache_images: Cache::new("images"),
        }
    }
}

struct CloudflareCredentials {
    key: String,
    account_id: String,
}

impl ImageEmbedder {
    fn get_credentials(&self) -> Option<CloudflareCredentials> {
        let key = env::var("OPTIONAL_CLOUDFLARE_API_KEY").ok()?;
        let account_id = env::var("OPTIONAL_CLOUDFLARE_ACCOUNT_ID").ok()?;
        Some(CloudflareCredentials { key, account_id })
    }

    pub fn supports_inlining_images(&self) -> bool {
        self.get_credentials().is_some()
    }

    pub fn embed_image(
        &self,
        post: &Post,
        image_url: &str,
        image_alt: &Option<String>,
    ) -> Result<EmbeddingResult, Error> {
        // Handle relative URLs by joining with the post's page URL
        let absolute_url = if image_url.starts_with("http") {
            image_url.to_string()
        } else {
            let base_url =
                Url::parse(&post.page_url).context("Failed to parse post URL as base URL")?;

            let joined_url = base_url
                .join(image_url)
                .context("Failed to join base URL with image URL")?;

            joined_url.to_string()
        };

        // If Cloudflare credentials are not set, return a text link
        if !self.supports_inlining_images() {
            // Extract file extension to check if it's an SVG
            let img_extension = absolute_url.split('.').next_back().unwrap_or("").to_lowercase();
            // Create anchor text for fallback
            let anchor_text = if let Some(image_alt) = image_alt {
                image_alt
            } else {
                image_url
            };

            // Determine prefix based on image type
            let prefix = if img_extension.contains("svg") {
                "Unsupported SVG image: "
            } else {
                "Image: "
            };
            return Ok(EmbeddingResult::Text(format!(
                "<a href=\"{}\">{}{}</a>",
                &absolute_url, prefix, &anchor_text
            )));
        }

        // Create a hash of the absolute URL for caching purposes
        // this also prevents using weird urls for local file system names
        let mut hasher = Sha256::new();
        hasher.update(absolute_url.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        let id = format!("{}-{}", post.id, &hash[..8]);

        Ok(EmbeddingResult::Image(ImageEmbedding {
            id,
            old_url: absolute_url,
            image_bytes: vec![],
        }))
    }

    pub async fn download_image(&self, image_embedding: &mut ImageEmbedding) -> Result<(), Error> {
        if let Some(cached) = self.cache_images.get(&image_embedding.id)? {
            image_embedding.image_bytes = cached;
            return Ok(());
        }

        // at this point credentials should always be available, as otherwise download_image is skipped
        let CloudflareCredentials {
            key: cloudflare_key,
            account_id: cloudflare_account_id,
        } = self
            .get_credentials()
            .expect("no Cloudflare credentials available");

        // Construct the HTML to render with Cloudflare. by default html and body have padding and the image is offset
        let html = format!("{}<img src=\"{}\">", "<style>* { margin: 0; padding: 0; } body { margin: 0; padding: 0; overflow: hidden; } img { display: block; width: 100%; height: auto; }</style>", image_embedding.old_url);
        println!("HTML: {}", html);

        let response = self
            .client
            .post(format!(
                "https://api.cloudflare.com/client/v4/accounts/{}/browser-rendering/screenshot",
                cloudflare_account_id
            ))
            .header("Authorization", format!("Bearer {}", cloudflare_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "html": html,
                "screenshotOptions": {
                    "omitBackground": false,
                    "fullPage": true
                },
                // fix for issue: if we just take a normal screenshot the viewport is larger than the actual image and we get large margins around it. instead, set viewport height to 1 and fullPage to true to perfectly capture the full image height only.
                "viewport": {
                    "width": 640,
                    "height": 1
                }
            }))
            .send()
            .await
            .context("Failed to send request to Cloudflare API")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Cloudflare API returned error: {}\n{}",
                response.status(),
                response.text().await?
            ));
        }

        let image_bytes = response
            .bytes()
            .await
            .context("Failed to get image bytes from Cloudflare API")?
            .to_vec();

        // Cache the image bytes
        self.cache_images.set(&image_embedding.id, &image_bytes)?;
        image_embedding.image_bytes = image_bytes;

        Ok(())
    }
}
