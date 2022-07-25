use std::sync::atomic::{AtomicUsize, Ordering};

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    id: String,
    title: String,
    overview: String,
}

#[tokio::main]
async fn main() {
    let documents = std::include_bytes!("../movies.json");
    let documents: Vec<Document> = serde_json::from_reader(documents.as_ref()).unwrap();
    let client = reqwest::Client::builder();
    // let client = client.http2_prior_knowledge();

    let client = client.build().unwrap();
    let client = &client;

    let theorical_max = &AtomicUsize::new(0);
    let score = &AtomicUsize::new(0);

    futures::stream::iter(documents.into_iter())
        .for_each_concurrent(16_000, |document| async move {
            let text = document.overview;
            for (i, _) in text.char_indices().take(30) {
                let query = &text[..i];
                let res = Box::pin(
                    futures::stream::repeat_with(|| async {
                        client
                            .clone()
                            .get(format!("http://localhost:3000/search?q={}", query))
                            .send()
                            .await
                    })
                    .filter_map(|res| async { res.await.ok() }),
                )
                .next()
                .await
                .unwrap();
                let res = res.json::<Value>().await.unwrap();
                if let Some(hits) = res["results"].as_array() {
                    if let Some(pos) = hits.iter().position(|doc| doc["id"] == json!(document.id)) {
                        score.fetch_add(10 - pos, Ordering::Relaxed);
                    }
                }
                theorical_max.fetch_add(10, Ordering::Relaxed);
            }
        })
        .await;

    println!("Got a score of {:?} on {:?}", score, theorical_max);
}
