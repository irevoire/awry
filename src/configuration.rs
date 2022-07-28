use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use urlencoding::encode;

#[derive(Debug, Deserialize)]
pub struct Configuration {
    verb: Verb,
    url: String,
    body: Option<String>,
    result: String,
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verb {
    Get,
    Post,
}

impl Configuration {
    pub async fn search(&self, client: Client, query: &str) -> reqwest::Result<reqwest::Response> {
        match self.verb {
            Verb::Get => {
                client
                    .get(self.url.replace("{}", &encode(query)))
                    .send()
                    .await
            }
            Verb::Post => {
                client
                    .post(&self.url)
                    .body(self.body.as_ref().unwrap().replace("{}", query))
                    .send()
                    .await
            }
        }
    }

    pub fn extract_ids(&self, result: &Value) -> Vec<Value> {
        if let Some(result) = result.get(&self.result) {
            result
                .as_array()
                .unwrap()
                .into_iter()
                .map(|doc| doc["id"].clone())
                .collect()
        } else {
            println!(
                "configuration error: {}",
                serde_json::to_string_pretty(&result).unwrap()
            );
            Vec::new()
        }
    }
}
