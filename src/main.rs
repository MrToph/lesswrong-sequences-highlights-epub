use clap::Parser;
use lesswrong_sequences_highlights_epub::{
    ai::{AiClient, AnnotatedPostWithComments},
    epub::Epub,
    lesswrong::LessWrongApi,
};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Generate EPUB from LessWrong posts with AI summaries"
)]
struct Args {
    /// Space-separated list of LessWrong post IDs
    #[clap(value_parser, num_args = 0..)]
    post_ids: Vec<String>,

    /// Output file path
    #[clap(short, long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let args = Args::parse();
    // if no post ids provided, build the sequences
    let is_sequences = args.post_ids.is_empty();

    let post_ids = if is_sequences {
        SEQUENCES_POST_IDS
    } else {
        &args.post_ids.iter().map(|s| s.as_str()).collect::<Vec<_>>()
    };

    let api = LessWrongApi::default();

    let mut posts = Vec::with_capacity(post_ids.len());
    for id in post_ids {
        let post = api.get_post_and_comments(id).await?;
        println!("Retrieved post: {}", post.post.title);
        println!("Comments count: {}", post.comments.len());
        posts.push(post);
    }

    let ai = AiClient::default();
    let mut annotated_posts = Vec::with_capacity(posts.len());
    for post in posts.drain(..) {
        println!("Creating POST summary for {}", &post.post.title);
        let summary = ai.summarize_post(&post.post).await?;
        println!("Creating COMMENTS summary for {}", &post.post.title);
        let comments_summary = ai.summarize_comments(&post).await?;
        annotated_posts.push(AnnotatedPostWithComments::new(
            post,
            summary,
            comments_summary,
        ));
    }

    let mut epub = Epub::default();
    // Determine output filename based on arguments and post IDs
    let output_path = match args.output {
        Some(path) => path,
        None => {
            if is_sequences {
                PathBuf::from("sequences-highlights.epub")
            } else {
                PathBuf::from(annotated_posts.first().unwrap().post.slug.clone() + ".epub")
            }
        }
    };

    let (title, author) = if is_sequences {
        (None, None)
    } else {
        (
            Some(annotated_posts.first().unwrap().post.title.clone()),
            Some(annotated_posts.first().unwrap().post.author.clone()),
        )
    };
    epub.set_metadata(title, author, is_sequences)?;

    for post in annotated_posts {
        epub.add_post(&post).await?;
    }

    let output = epub.generate()?;
    std::fs::write(output_path, output)?;

    Ok(())
}

// Default post IDs for the sequences if none provided
const SEQUENCES_POST_IDS: &[&str] = &[
    "46qnWRSR7L2eyNbMA",
    "RcZCwxFiZzE6X7nsv",
    "PBRWb2Em5SNeWYwwB",
    "fhEPnveFhb9tmd7Pe",
    "5JDkW4MYXit2CquLs",
    "3nZMgRTfFEfHp34Gb",
    "wCqfCLs8z5Qw4GbKS",
    "teaxCFgtmCQ3E9fy8",
    "7ZqGiPHTpiDMwqMN2",
    "34XxbRFe54FycoCDw",
    "SFZoEBpLo9frSJGkc",
    "HYWhKXRsMAyvRKRYz",
    "TGux5Fhcd7GmTfNGC",
    "dHQkDNMhj692ayx78",
    "nYkMLFpx77Rz3uo9c",
    "2MD3NMLBPCqPfnfre",
    "dLJv2CoRCgeC2mPgj",
    "CEGnJBHmkcwPTysb7",
    "rmAbiEKQDpDnZzcRf",
    "AdYdLP2sRqPMoe8fb",
    "9weLK2AJ9JEt2Tt8f",
    "a7n8GdKiAZRX86T5A",
    "6s3xABaXKPdFwA3FS",
    "fhojYBGGiYAFcryHZ",
    "nj8JKFoLSMEmD3RGp",
    "mnS2WYLCGJP2kQkRn",
    "jiBFC7DcCrZjGmZnJ",
    "5yFRd3cjLpm3Nd6Di",
    "XTXWPQSEgoMkAupKt",
    "QkX2bAkwG2EpGvNug",
    "CPP2uLcaywEokFKQG",
    "WQFioaudEH8R7fyhm",
    "wzxneh7wxkdNYNbtB",
    "xTyuQ3cgsPjifr7oj",
    "5bJyRMZzwMov5u3hW",
    "wustx45CPL5rZenuo",
    "WBdvyyHLdxZSAMmoz",
    "Mc6QcrsbH5NRXbCRX",
    "895quRDaK6gR2rM82",
    "2jp98zdLo898qExrr",
    "kpRSCH7ALLcb6ucWM",
    "ZTRiSNmeGQK8AkdN2",
    "yA4gF5KrboK2m2Xu7",
    "HLqWn5LASfhhArZ7w",
    "sSqoEw9eRP2kPKLCz",
    "SGR4GxFK7KmW7ckCB",
    "pGvyqAQw6yqTjpKf4",
    "ur9TCRnHJighHmLCW",
    "DoLQN5ryZ9XkZjq5h",
    "Nu3wa6npK4Ry66vFp",
];
