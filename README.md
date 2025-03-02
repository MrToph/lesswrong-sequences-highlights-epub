# LessWrong Sequences Highlights epub

This is an epub version of the [LessWrong's Highlights from the Sequences](https://www.lesswrong.com/highlights).
**Each chapter also includes AI-generated summaries and a summary of the comments.**

The `sequences-highlights.epub` file can be downloaded [here](https://github.com/MrToph/lesswrong-sequences-highlights-epub/raw/refs/heads/main/sequences-highlights.epub).

All images in a post are embedded into the epub using [Cloudflare's `capture-screenshot` API](https://developers.cloudflare.com/browser-rendering/rest-api/screenshot-endpoint/).
This also indirectly supports `.svg` images by pixelizing them.
(Inlining SVGs breaks [sendtokindle](https://www.amazon.com/sendtokindle) resulting in "E999 - Send to Kindle Internal Error").

> [!TIP]
> This code can be used to create an epub with AI post & comment summaries for _any_ collection of lesswrong posts by passing the post IDs as arguments to the CLI.
> 
> ```bash
> cargo run -- <comma-separated-post-ids> [--output optional-file-name.epub]
> ```

# Issues

- [ ] Footnotes are displayed as `^1^` and not hyperlinked. In a post's HTML it's displayed as `<sup>1</sup>`, in the Markdown as `^1^`. It's not being properly converted by the `markdown` crate.

# Development

```bash
# for AI calls, any OpenAI-compatible provider will work. (openrouter, venice.ai)
# for embedding the post's images, a Cloudflare browser rendering API key is required. If it's not present the images are simply replaced with a link to the image.
cp example.env .env

cargo build
cargo run
```
