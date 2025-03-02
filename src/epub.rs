use anyhow::Result;
use chrono::{DateTime, Utc};
use epub_builder::{EpubContent, ReferenceType, ZipLibrary};
use handlebars::Handlebars;
use lesswrong_api::Post;
use lol_html::{element, html_content::ContentType, rewrite_str, RewriteStrSettings};
use serde_json::json;

use crate::{
    ai::AnnotatedPostWithComments,
    image_embedder::{EmbeddingResult, ImageEmbedder},
};

fn format_date(date: DateTime<Utc>) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn words_to_read_time(words: i64) -> String {
    // reading speed of words per minute. then: minutes = words / (130 words / minute)
    let wpm = 130;
    let minutes_round_up = (words + wpm - 1) / wpm;
    format!("{}min", minutes_round_up)
}

pub struct Epub {
    builder: epub_builder::EpubBuilder<ZipLibrary>,
    image_embedder: ImageEmbedder,
}

impl Default for Epub {
    fn default() -> Self {
        let builder =
            epub_builder::EpubBuilder::new(epub_builder::ZipLibrary::new().unwrap()).unwrap();

        Self {
            builder,
            image_embedder: ImageEmbedder::default(),
        }
    }
}

impl Epub {
    pub fn set_metadata(&mut self, title: Option<String>, author: Option<String>, use_cover_image: bool) -> Result<&mut Self> {
        let stylesheet = std::fs::read("epub_resources/stylesheet.css")?;

        // Default title if none provided
        let title = title.unwrap_or_else(|| "LessWrong Sequences Highlights".to_string());
        let author = author.unwrap_or_else(|| "Eliezer Yudkowsky".to_string());

        // Kindle shows <bold>filename</bold><br/><small>author</small>. if no cover image
        self.builder
            .stylesheet(stylesheet.as_slice())
            .map_err(|e| anyhow::anyhow!(e))?
            .metadata("author", &author)
            .map_err(|e| anyhow::anyhow!(e))?
            .metadata("title", &title)
            .map_err(|e| anyhow::anyhow!(e))?
            .epub_version(epub_builder::EpubVersion::V30)
            .set_title(&title);

        // Use the provided cover image or fall back to the default one
        if use_cover_image {
            let cover_path = "epub_resources/cover.jpg".to_string();
            let cover_image = std::fs::read(&cover_path)?;
            self.builder
                .add_cover_image("cover.jpg", cover_image.as_ref() as &[u8], "image/jpeg")
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        Ok(self)
    }

    pub async fn add_post(&mut self, post: &AnnotatedPostWithComments) -> Result<&mut Self> {
        // convert the markdown to html instead of using content_html because the HTML output is cleaner this way. epub html also errors on some tags that are not closed like <hr>
        let post_html = markdown::to_html(&post.post.content_markdown);
        let (post_html, replacements) = self.try_inline_images(&post.post, post_html).await?;

        let post_summary_html = markdown::to_html(&post.post_summary);
        let comments_summary_html = markdown::to_html(&post.comments_summary);

        let template = std::fs::read_to_string("epub_resources/post.html.hbs")?;
        let reg = Handlebars::new();
        let xhtml = reg.render_template(
            &template,
            &json!({"title": post.post.title, "body": post_html, "date": format_date(post.post.date), "author": post.post.author, "read_time": words_to_read_time(post.post.word_count), "post_summary": post_summary_html, "comments_summary": comments_summary_html }),
        )?;

        self.builder
            .add_content(
                EpubContent::new(format!("{}.xhtml", post.post.slug), xhtml.as_bytes())
                    .title(post.post.title.clone())
                    .reftype(ReferenceType::Text),
            )
            .map_err(|e| anyhow::anyhow!(e))?;

        replacements
            .into_iter()
            .filter_map(|r| {
                if let EmbeddingResult::Image(embedded_image) = r {
                    Some(embedded_image)
                } else {
                    None
                }
            })
            .try_for_each(|embedded_image| {
                self.builder
                    .add_resource(
                        // apparently Cloudflare's API returns all images in png format, even though their example says webp
                        format!("{}.png", embedded_image.id),
                        embedded_image.image_bytes.as_slice(),
                        "image/png",
                    )
                    // Convert success value to () to match try_for_each's expected return type
                    .map(|_| ())
                    .map_err(|e| anyhow::anyhow!(e))
            })?;
        Ok(self)
    }

    pub fn generate(&mut self) -> Result<Vec<u8>> {
        let mut output = Vec::<u8>::new();

        self.builder
            .generate(&mut output)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(output)
    }

    async fn try_inline_images(
        &self,
        post: &Post,
        html: String,
    ) -> Result<(String, Vec<EmbeddingResult>)> {
        let mut replaced: Vec<EmbeddingResult> = vec![];
        let element_content_handlers = vec![element!("img[src]", |el| {
            let img_src = el.get_attribute("src").unwrap().trim().to_string();
            let img_alt = el
                .get_attribute("alt")
                .unwrap_or_default()
                .trim()
                .to_string();
            let img_alt = if img_alt.is_empty() {
                None
            } else {
                Some(img_alt)
            };
            let embedding = self.image_embedder.embed_image(post, &img_src, &img_alt)?;

            match &embedding {
                EmbeddingResult::Text(replaced_html) => {
                    el.after(replaced_html, ContentType::Html);
                }
                EmbeddingResult::Image(embedded_image) => {
                    el.after(
                        &format!("<img src=\"{}.png\" />", embedded_image.id),
                        ContentType::Html,
                    );
                }
            }
            el.remove();

            replaced.push(embedding);
            Ok(())
        })];
        let output = rewrite_str(
            html.as_str(),
            RewriteStrSettings {
                element_content_handlers,
                ..RewriteStrSettings::new()
            },
        )
        .unwrap();

        for result in &mut replaced.iter_mut() {
            match result {
                EmbeddingResult::Image(embedding) => {
                    self.image_embedder.download_image(embedding).await?;
                }
                EmbeddingResult::Text(_) => {}
            }
        }

        Ok((output, replaced))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lesswrong_api::Post;

    #[tokio::test]
    async fn replaces_multiple_images_as_text() -> Result<()> {
        let epub = Epub::default();

        let post = Post {
            id: "test-epub".to_string(),
            page_url: "https://example.com/test-post".to_string(),
            ..Post::default()
        };

        let input = r#"
            <article>
                <img src="first.svg" alt="Diagram">
                <p>Text</p>
                <img class="flex" src="second.png">
                <img alt="" src="third.png">
            </article>
        "#;

        let (output, _replaced) = epub.try_inline_images(&post, input.to_string()).await?;

        assert!(output.contains(
            r#"<a href="https://example.com/first.svg">Unsupported SVG image: Diagram</a>"#
        ));
        assert!(
            output.contains(r#"<a href="https://example.com/second.png">Image: second.png</a>"#)
        );
        assert!(output.contains(r#"<a href="https://example.com/third.png">Image: third.png</a>"#));

        Ok(())
    }
}
