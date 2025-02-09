use anyhow::Result;
use chrono::{DateTime, Utc};
use epub_builder::{EpubContent, ReferenceType, ZipLibrary};
use handlebars::Handlebars;
use lol_html::{element, html_content::ContentType, rewrite_str, RewriteStrSettings};
use serde_json::json;

use crate::ai::AnnotatedPostWithComments;

#[allow(dead_code)]
fn try_get_image_bytes(image_url: &str) -> Option<Vec<u8>> {
    if image_url.starts_with("http") {
        todo!("fetching remote images not supported yet")
    } else {
        let bytes = std::fs::read(image_url).ok()?;
        Some(bytes)
    }
}

#[allow(dead_code)]
fn format_date(date: DateTime<Utc>) -> String {
    date.format("%Y-%m-%d").to_string()
}

#[allow(dead_code)]
fn words_to_read_time(words: i64) -> String {
    // reading speed of words per minute. then: minutes = words / (100 words / minute)
    let wpm = 130;
    let minutes_round_up = (words + wpm - 1) / wpm;
    format!("{}min", minutes_round_up)
}

fn try_inline_images(html: String) -> Result<(String, Vec<String>)> {
    let mut replaced: Vec<String> = vec![];
    let element_content_handlers = vec![
        // Rewrite insecure hyperlinks
        element!("img[src]", |el| {
            // TODO: some URLs are relative. need to resolve them relative to post.page_url
            let img_src = el.get_attribute("src").unwrap();
            let img_extension = img_src.split(".").last().unwrap();

            // some images are <img alt="" ..>
            let mut anchor_text = el.get_attribute("alt").unwrap_or_default();
            if anchor_text.trim().is_empty() {
                anchor_text = img_src.clone();
            }

            let prefix = if img_extension.contains("svg") {
                "Unsupported SVG image: "
            } else {
                "Image: "
            };
            el.after(
                &format!("<a href=\"{}\">{}{}</a>", &img_src, prefix, &anchor_text),
                ContentType::Html,
            );
            el.remove();

            replaced.push(img_src);
            Ok(())
        }),
    ];
    let output = rewrite_str(
        html.as_str(),
        RewriteStrSettings {
            element_content_handlers,
            ..RewriteStrSettings::new()
        },
    )
    .unwrap();

    Ok((output, replaced))
}

pub struct Epub {
    builder: epub_builder::EpubBuilder<ZipLibrary>,
}

impl Default for Epub {
    fn default() -> Self {
        let builder =
            epub_builder::EpubBuilder::new(epub_builder::ZipLibrary::new().unwrap()).unwrap();

        Self { builder }
    }
}

impl Epub {
    pub fn set_metadata(&mut self) -> Result<&mut Self> {
        let stylesheet = std::fs::read("epub_resources/stylesheet.css")?;

        // Kindle shows <bold>filename</bold><br/><small>author</small>. if no cover image
        self.builder
            .stylesheet(stylesheet.as_slice())
            .map_err(|e| anyhow::anyhow!(e))?
            .metadata("author", "Eliezer Yudkowsky")
            .map_err(|e| anyhow::anyhow!(e))?
            .metadata("title", "LessWrong Sequences Highlights")
            .map_err(|e| anyhow::anyhow!(e))?
            .epub_version(epub_builder::EpubVersion::V30)
            .set_title("LessWrong Sequences Highlights");

        let cover_image = try_get_image_bytes("epub_resources/cover.jpg");
        if let Some(cover_image) = cover_image {
            self.builder
                .add_cover_image("cover.jpg", cover_image.as_ref() as &[u8], "image/jpeg")
                .map_err(|e| anyhow::anyhow!(e))?;
        }

        // needs valid xhtml or epub breaks
        // self.builder
        //     .add_content(
        //         EpubContent::new("cover.xhtml", "<xhtml />".as_bytes())
        //             .title("Cover")
        //             .reftype(ReferenceType::Cover),
        //     )
        //     .map_err(|e| anyhow::anyhow!(e))?;

        // self.builder
        //     .add_content(
        //         EpubContent::new("title.xhtml", "<xhtml />".as_bytes())
        //             .title("Title")
        //             .reftype(ReferenceType::TitlePage),
        //     )
        //     .map_err(|e| anyhow::anyhow!(e))?;

        // self.builder.inline_toc();

        Ok(self)
    }

    pub fn add_post(&mut self, post: &AnnotatedPostWithComments) -> Result<&mut Self> {
        // convert the markdown to html instead of using content_html because the HTML output is cleaner this way. epub html also errors on some tags that are not closed like <hr>
        let post_html = markdown::to_html(&post.post.content_markdown);
        let (post_html, _) = try_inline_images(post_html)?;

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
        Ok(self)
    }

    pub fn generate(&mut self) -> Result<Vec<u8>> {
        let mut output = Vec::<u8>::new();

        self.builder
            .generate(&mut output)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_multiple_images() -> Result<()> {
        let input = r#"
            <article>
                <img src="first.svg" alt="Diagram">
                <p>Text</p>
                <img class="flex" src="second.png">
                <img alt="" src="third.png">
            </article>
        "#;

        let (output, replaced) = try_inline_images(input.to_string())?;

        assert_eq!(replaced, vec!["first.svg", "second.png", "third.png"]);
        assert!(output.contains(r#"<a href="first.svg">Unsupported SVG image: Diagram</a>"#));
        assert!(output.contains(r#"<a href="second.png">Image: second.png</a>"#));
        assert!(output.contains(r#"<a href="third.png">Image: third.png</a>"#));
        Ok(())
    }
}
