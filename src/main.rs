mod configuration;

use configuration::Configuration;
use indicatif::{MultiProgress, ProgressBar};
use tokio::sync::mpsc;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    id: String,
    title: String,
    overview: String,
}

fn leak<T>(a: T) -> &'static T {
    Box::leak(Box::new(a))
}

#[tokio::main]
async fn main() {
    let conf_file = std::env::args()
        .nth(1)
        .expect("Need a configuration to run");
    let conf = std::fs::read_to_string(&conf_file).unwrap();
    let conf: Configuration = toml::de::from_str(&conf).unwrap();
    let conf = leak(conf);

    let documents = std::include_bytes!("../movies.json");
    let documents: Vec<Document> = serde_json::from_reader(documents.as_ref()).unwrap();
    let documents = leak(documents);

    let client = reqwest::Client::builder();
    let client = leak(client.build().unwrap());

    let (sender, mut receiver) = mpsc::channel(100);

    let limit = 50;

    let multibar = MultiProgress::new();
    let document_bar = ProgressBar::new(50);
    let words_bar = ProgressBar::new(10);

    multibar.add(document_bar.clone());
    multibar.add(words_bar.clone());

    document_bar.println("Documents");
    words_bar.println("Words");

    tokio::spawn(async move {
        let sender = &sender.clone();
        let document_bar = &document_bar.clone();
        let words_bar = &words_bar.clone();

        futures::stream::iter(documents.into_iter().take(limit))
            .for_each_concurrent(1_000, move |document| async {
                let text = &document.overview;
                let words_indices: Vec<_> = std::iter::once(0)
                    .chain(text.match_indices(' ').map(|(i, s)| i + s.len()))
                    .chain(std::iter::once(text.len()))
                    .collect();

                let iter = words_indices.windows(4);
                words_bar.set_length(words_bar.length().unwrap_or_default() + iter.len() as u64);

                for words_indices in iter {
                    let start = words_indices[0];
                    let end = words_indices[3];

                    for end in text[start..end]
                        .char_indices()
                        .skip(1)
                        .map(|(i, _)| i + start)
                    {
                        let query = &text[start..end];
                        // println!("{query}");
                        let res = Box::pin(
                            futures::stream::repeat_with(|| async {
                                conf.search(client.clone(), query).await
                            })
                            .map(|res| async {
                                let res = res.await;
                                // TODO: ensure this is an unauthorized error before ignoring it
                                // and introduce a timer or something instead of spamming
                                if res.is_err() {
                                    println!("Error");
                                }
                                res
                            })
                            .filter_map(|res| async { res.await.ok() }),
                        )
                        .next()
                        .await
                        .unwrap();
                        if res.status() != 200 {
                            println!("error with query: {query}");
                        }
                        let res = res.json::<Value>().await.unwrap();
                        let ids = conf.extract_ids(&res);
                        if let Some(score) = ids.iter().position(|id| id == &json!(document.id)) {
                            sender
                                .clone()
                                .send(Request::new(end, text, score))
                                .await
                                .unwrap();
                        }
                    }
                    words_bar.inc(1);
                }
                document_bar.inc(1);
            })
            .await;
    });

    // if we have an error now, let’s just ignore it
    if let Err(e) = multibar.clear() {
        println!("An error happened with the loading bar; {e}");
    }

    let mut score = [0; 3];
    let mut theorical_max = [0; 3];

    while let Some(res) = receiver.recv().await {
        let s = 10 - res.score;

        for kind in res.kinds {
            score[kind as usize] += s;
            theorical_max[kind as usize] += 10;
        }
        score[2] += s;
        theorical_max[2] += 10;
    }

    let scores = score
        .into_iter()
        .zip(theorical_max.into_iter())
        .map(|(score, max)| score * 100 / max)
        .collect::<Vec<_>>();

    println!("{:8} | {:8} | {:8}", "exact", "prefix", "total");
    println!("{:8} | {:8} | {:8}", scores[0], scores[1], scores[2]);
}

#[derive(Debug, Clone)]
pub struct Request {
    score: usize,
    kinds: Vec<Kind>,
}

impl Request {
    pub fn new(end: usize, origin: &str, score: usize) -> Self {
        let mut kinds = Vec::new();

        // we end in the middle of a word
        if matches!(origin.bytes().nth(end + 1), None | Some(b' ')) {
            kinds.push(Kind::Prefix);
        }

        // we were not a prefix or a suffix, thus we’re an exact match
        if kinds.is_empty() {
            kinds.push(Kind::Exact);
        }

        Self { score, kinds }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Exact = 0,
    Prefix,
    Total,
    // Typo,
}
