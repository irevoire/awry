mod configuration;

use configuration::Configuration;
use tokio::sync::mpsc;

use std::sync::atomic::{AtomicUsize, Ordering};

use futures::StreamExt;
use reqwest::Client;
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

    let main_op = tokio::spawn(async move {
        let sender = &sender.clone();
        futures::stream::iter(documents.into_iter())
            .for_each_concurrent(1_000, move |document| async {
                let text = &document.overview;
                for (i, _) in text.char_indices().take(30) {
                    let query = &text[..i];
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
                    if let Some(pos) = ids.iter().position(|id| id == &json!(document.id)) {
                        sender.clone().send(pos).await.unwrap();
                    }
                }
            })
            .await;
    });

    let mut score = 0;
    let mut theorical_max = 0;

    while let Some(res) = receiver.recv().await {
        score += 10 - res;
        theorical_max += 10;
    }

    println!("Got a score of {:?} on {:?}", score, theorical_max);
}

#[derive(Debug, Clone)]
pub struct Request {
    score: usize,
    kinds: Vec<Kind>,
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Exact,
    Prefix,
    Suffix,
    Typo,
}
