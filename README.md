# LessWrong Sequences Highlights epub

This is an epub version of the [LessWrong's Highlights from the Sequences](https://www.lesswrong.com/highlights).
**Each chapter also includes AI-generated summaries and a summary of the comments.**

The `sequences-highlights.epub` file can be downloaded [here](https://github.com/MrToph/lesswrong-sequences-highlights-epub/raw/refs/heads/main/sequences-highlights.epub).

> [!TIP]
> This code can be used to create an epub with AI post & comment summaries for _any_ lesswrong posts by adjusting the post IDs in [`main.rs`](./src/main.rs).


# Issues

- [ ] Footnotes are displayed as `^1^` and not hyperlinked. In a post's HTML it's displayed as `<sup>1</sup>`, in the Markdown as `^1^`. It's not being properly converted by the `markdown` crate.
- [ ] SVG images break [sendtokindle](https://www.amazon.com/sendtokindle), resulting in an "E999 - Send to Kindle Internal Error". **SVG images are replaced with a link to the image instead.**
- [ ] General image support not implemented. (epubs need to inline them.) **A link to the image is displayed instead.**

# Development

```bash
# for the AI calls, any OpenAI-compatible provider will work. (openrouter, venice.ai)
cp example.env .env

cargo build
cargo run
```
