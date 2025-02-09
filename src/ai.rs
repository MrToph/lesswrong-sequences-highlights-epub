use crate::{
    cache::Cache, lesswrong::PostWithComments, sort_comments::sort_comments_by_score_depth_first,
};
use anyhow::Result;
use lesswrong_api::{Comment, Post};
use openai::{
    chat::{
        ChatCompletion, ChatCompletionChoice, ChatCompletionGeneric, ChatCompletionMessage,
        ChatCompletionMessageRole, VeniceParameters,
    },
    Credentials, OpenAiError,
};
use regex::Regex;
use std::{collections::HashMap, env};

pub struct AnnotatedPostWithComments {
    pub post: Post,
    pub comments: HashMap<String, Comment>,
    pub post_summary: String,
    pub comments_summary: String,
}

impl AnnotatedPostWithComments {
    pub fn new(post: PostWithComments, post_summary: String, comments_summary: String) -> Self {
        Self {
            post: post.post,
            comments: post.comments,
            post_summary,
            comments_summary,
        }
    }
}

fn remove_think_tags(input: &str) -> String {
    // (?is) makes the regex case-insensitive and `.` also matches newlines
    let re = Regex::new(r"(?is)<think[^>]*?>.*?</think[^>]*?>").unwrap();
    re.replace_all(input, "").to_string()
}

pub struct AiClient {
    cache_post: Cache<String>,
    cache_comments: Cache<String>,
    credentials: Credentials,
    model: String,
}

impl Default for AiClient {
    fn default() -> Self {
        let credentials = Credentials::from_env();

        Self {
            cache_post: Cache::new("ai-posts"),
            cache_comments: Cache::new("ai-comments"),
            credentials,
            model: env::var("OPENAI_MODEL").unwrap_or_else(|_| panic!("OPENAI_MODEL not set")),
        }
    }
}

impl AiClient {
    async fn create_summarize_post_completion(
        &self,
        post: &Post,
    ) -> Result<ChatCompletionGeneric<ChatCompletionChoice>, OpenAiError> {
        let post_messages = vec![
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: Some(
                "You are an expert of distilling complex rationalist topics to a concise summary. Approach topics with an intellectual but approachable tone, NOT USING LISTS UNLESS NECESSARY and strategically to organize complex ideas. Incorporate engaging narrative techniques like anecdotes, concrete examples, and thought experiments to draw the reader into the intellectual exploration. Maintain an academic rigor while simultaneously creating a sense of collaborative thinking, as if guiding the reader through an intellectual journey. Use precise language that is simultaneously scholarly and accessible, avoiding unnecessary jargon while maintaining depth of analysis. Don't waste too many words with framing and setup. Optimize for quick readability and depth. Use formatting techniques like bold, italics, and call outs (quotation blocks and such) for specific definitions and interesting terms. This will also break up the visual pattern, making it easier for the reader to stay oriented and anchored.  Don't hesitate to use distal connection, metaphor, and analogies as well, particularly when you notice meta-patterns emerging. A good metaphor is the pinnacle of Coherence. Stylistically, use a variety of techniques to create typographic scaffolding and layered information. With this in mind, summarize the main points of the following LessWrong article keeping it under about 200 words: DO NOT BE REPETITIVE.".to_string(),
            ),
            ..Default::default()
        },
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: Some(post.content_markdown.clone()),
            ..Default::default()
        },
    ];
        let post_completion = ChatCompletion::builder(&self.model, post_messages)
            .venice_parameters(VeniceParameters {
                include_venice_system_prompt: false,
            })
            .credentials(self.credentials.clone())
            .create();

        post_completion.await
    }

    async fn create_summarize_comments_completion(
        &self,
        post: &PostWithComments,
    ) -> Result<ChatCompletionGeneric<ChatCompletionChoice>, OpenAiError> {
        let comments = sort_comments_by_score_depth_first(&post.comments, 100);
        let comments = comments
            .iter()
            .map(|c| {
                let score = c.base_score;
                let content = c.content_markdown.clone();
                format!("<comment><score>{}</score>: {}</comment>", score, content).to_string()
            })
            .collect::<Vec<String>>()
            .join("\n");

        let post_messages = vec![
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: Some(
                "You are an expert of distilling complex rationalist topics to a concise summary. Approach topics with an intellectual but approachable tone, NOT USING LISTS UNLESS NECESSARY and strategically to organize complex ideas. Incorporate engaging narrative techniques like anecdotes, concrete examples, and thought experiments to draw the reader into the intellectual exploration. Maintain an academic rigor while simultaneously creating a sense of collaborative thinking, as if guiding the reader through an intellectual journey. Use precise language that is simultaneously scholarly and accessible, avoiding unnecessary jargon while maintaining depth of analysis. Don't waste too many words with framing and setup. Optimize for quick readability and depth. Use formatting techniques like bold, italics, and call outs (quotation blocks and such) for specific definitions and interesting terms. This will also break up the visual pattern, making it easier for the reader to stay oriented and anchored.  Don't hesitate to use distal connection, metaphor, and analogies as well, particularly when you notice meta-patterns emerging. A good metaphor is the pinnacle of Coherence. Stylistically, use a variety of techniques to create typographic scaffolding and layered information. With this in mind, summarize THE DISCUSSION IN THE COMMENTS presented here keeping it under about 200 words. A comment can contain a score, give more importance to higher scores BUT DO NOT EXPLICITLY MENTION THE SCORES. Comments can also be replies to previous comments, all comments are provided depth-first. DO NOT BE REPETITIVE. DO NOT SUMMARIZE THE POST ITSELF, IT IS ONLY PROVIDED AS CONTEXT.".to_string(),
            ),
            ..Default::default()
        },
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: Some(format!("<post>{}</post><comments>{}</comments>", post.post.content_markdown.clone(), comments)),
            ..Default::default()
        },
    ];
        let post_completion = ChatCompletion::builder(&self.model, post_messages)
            .venice_parameters(VeniceParameters {
                include_venice_system_prompt: false,
            })
            .credentials(self.credentials.clone())
            .create();

        post_completion.await
    }

    pub async fn summarize_post(&self, post: &Post) -> Result<String> {
        if let Some(cached) = self.cache_post.get(&post.id)? {
            return Ok(cached);
        }
        let request = self.create_summarize_post_completion(post).await?;

        let response = request.choices[0]
            .message
            .content
            .clone()
            .unwrap_or_default();
        let response = remove_think_tags(&response).trim().to_string();

        self.cache_post.set(&post.id, &response)?;
        Ok(response)
    }

    pub async fn summarize_comments(&self, post: &PostWithComments) -> Result<String> {
        if let Some(cached) = self.cache_comments.get(&post.post.id)? {
            return Ok(cached);
        }
        let request = self.create_summarize_comments_completion(post).await?;

        let response = request.choices[0]
            .message
            .content
            .clone()
            .unwrap_or_default();
        let response = remove_think_tags(&response).trim().to_string();

        self.cache_comments.set(&post.post.id, &response)?;
        Ok(response)
    }
}

#[test]
fn test_remove_think_tags() {
    let cases = vec![
        ("<think>ok this is it</think>hello", "hello"),
        ("<thinking>this works too</think>hello", "hello"),
        (
            "<thinkAWDWADAWWAD>\nwadwa\nawdwadaw\nawdaw</think>hello",
            "hello",
        ),
    ];

    for (input, expected) in cases {
        assert_eq!(remove_think_tags(input), expected);
    }
}
