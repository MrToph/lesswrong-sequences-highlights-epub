use crate::cache::Cache;
use anyhow::Result;
use lesswrong_api::{Comment, LessWrongApiClient, Post};
use std::collections::HashMap;
pub struct LessWrongApi {
    client: LessWrongApiClient,
    cache_post: Cache<Post>,
    cache_comments: Cache<HashMap<String, Comment>>,
}
pub struct PostWithComments {
    pub post: Post,
    pub comments: HashMap<String, Comment>,
}

impl Default for LessWrongApi {
    fn default() -> Self {
        Self {
            client: LessWrongApiClient::default(),
            cache_post: Cache::new("posts"),
            cache_comments: Cache::new("comments"),
        }
    }
}

impl LessWrongApi {
    pub async fn get_post_and_comments(&self, id: &str) -> Result<PostWithComments> {
        let post = match self.cache_post.get(id)? {
            Some(post) => post,
            None => {
                let post = self.client.get_post(id).await?;
                self.cache_post.set(id, &post)?;
                post
            }
        };

        let comments = match self.cache_comments.get(id)? {
            Some(comments) => comments,
            None => {
                let comments = self.client.get_comments(id, 9999).await?;
                self.cache_comments.set(id, &comments)?;
                comments
            }
        };

        Ok(PostWithComments { post, comments })
    }
}
