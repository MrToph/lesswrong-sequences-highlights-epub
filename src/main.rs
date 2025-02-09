use lesswrong_sequences_highlights_epub::{
    ai::{AiClient, AnnotatedPostWithComments},
    epub::Epub,
    lesswrong::LessWrongApi,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let post_ids = vec![
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
    epub.set_metadata()?;

    for post in annotated_posts {
        epub.add_post(&post)?;
    }

    let output = epub.generate()?;
    std::fs::write("sequences-highlights.epub", output)?;

    Ok(())
}
