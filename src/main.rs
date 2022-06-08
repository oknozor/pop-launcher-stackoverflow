use anyhow::Result;
use pop_launcher_toolkit::launcher::{IconSource, Indice, PluginResponse, PluginSearchResult};
use pop_launcher_toolkit::plugin_trait::tracing::error;
use pop_launcher_toolkit::plugin_trait::{async_trait, PluginExt};
use pop_launcher_toolkit::plugins::{send, xdg_open};
use serde::Deserialize;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};
use htmlescape::decode_html;
use tokio::select;
use tokio::sync::mpsc::Receiver;
use tokio::sync::{broadcast, mpsc};

mod config;

// Our Plugin, holding the Stackoverflow search results
struct StackOverflowPlugin {
    items: Arc<Mutex<Vec<StackOverFlowPost>>>,
    search_tx: mpsc::Sender<Vec<StackOverFlowPost>>,
    interrupt: broadcast::Sender<()>,
    access_token: String,
}

#[async_trait]
impl PluginExt for StackOverflowPlugin {
    // Define the name of our plugin, pop-launcher will use
    // This internally, to initialize the plugin, write to the logfile at runtime etc.
    fn name(&self) -> &str {
        "stackoverflow"
    }

    // This will be called upon receiving pop-launcher search query
    async fn search(&mut self, query: &str) {
        let sender = self.search_tx.clone();
        let mut interrupt = self.interrupt.subscribe();

        let query = query.strip_prefix("stk ").map(str::to_string);
        if let Some(query) = query {
            // Avoid sending empty requests to github
            if query.trim().is_empty() {
                return;
            }

            let access_token = self.access_token.clone();
            tokio::spawn(async move {
                select! {
                    query_result = search_posts(&query, &access_token) => {
                        match query_result {
                            Ok(query_result) => sender.send(query_result.items).await.expect("Failed to send query result"),
                            Err(why) => error!("Failed to obtain query result from github: {why}")
                        }
                    }

                    Ok(()) = interrupt.recv() => {
                        // Just return from this future
                    }
                }
            });
        }
    }

    // This is called whenever the user ask to activate the current selected item
    async fn activate(&mut self, id: Indice) {
        {
            let items = self.items.lock().unwrap();

            // Get our stackoverflow post at the selected index
            // and open it in the browser with the `xdg_open` helper function.
            match items.get(id as usize) {
                Some(post) => xdg_open(&post.link),
                None => error!("Failed to get post at index {id}"),
            }
        }

        // We are done, tell pop-launcher to close the client
        self.respond_with(PluginResponse::Close).await;
    }

    async fn interrupt(&mut self) {
        let _ = self.interrupt.send(());
        {
            let mut search_results = self.items.lock().unwrap();
            search_results.clear();
        }
        self.respond_with(PluginResponse::Finished).await;
    }
}

async fn dispatch_search_result(
    search_rx: &mut Receiver<Vec<StackOverFlowPost>>,
    search_results: Arc<Mutex<Vec<StackOverFlowPost>>>,
) {
    while let Some(new_results) = search_rx.recv().await {
        // Wrap the mutex guard into a scope so we don't hold it across the async `send` method.
        let plugin_responses: Vec<PluginResponse> = {
            let mut search_results = search_results.lock().unwrap();
            *search_results = new_results;

            search_results
                .iter()
                .enumerate()
                .map(|(idx, entry)| entry.to_plugin_response(idx as u32))
                .collect()
        };

        for search_result in plugin_responses {
            send(&mut tokio::io::stdout(), search_result).await;
        }

        send(&mut tokio::io::stdout(), PluginResponse::Finished).await;
    }
}

// Deserializable structs to hold the stackoverflow api response
#[derive(Deserialize, Debug, PartialEq)]
struct StackOverFlowResponse {
    items: Vec<StackOverFlowPost>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct StackOverFlowPost {
    title: String,
    score: i32,
    link: String,
    tags: Vec<String>,
    is_answered: bool,
}

impl StackOverFlowPost {
    fn tags(&self) -> String {
        self.tags.join(", ")
    }

    fn to_plugin_response(&self, idx: u32) -> PluginResponse {
        PluginResponse::Append(PluginSearchResult {
            id: idx as u32,
            name: decode_html(&self.title.clone()).expect("Decode error").to_string(),
            description: decode_html(&self.tags()).expect("Decode error").to_string(),
            icon: if self.is_answered {
                Some(IconSource::Name(Cow::Owned("emblem-checked".to_string())))
            } else {
                Some(IconSource::Name(Cow::Owned("error".to_string())))
            },
            ..Default::default()
        })
    }
}

// Call the stackoverflow api with the provided `intitle` query param
async fn search_posts(intitle: &str, access_token: &str) -> Result<StackOverFlowResponse> {
    let response = ureq::get("https://api.stackexchange.com/2.3/search?")
        .query("access_token", access_token)
        .query("key", "4Muhe4yLUPuS2xtKQzlRhQ((")
        .query("page", "1")
        .query("pagesize", "8")
        .query("order", "desc")
        .query("sort", "activity")
        .query("site", "stackoverflow")
        .query("intitle", &format!("\"{intitle}\""))
        .call()?;

    response.into_json().map_err(Into::into)
}

// Spawn tokio runtime and run our plugin
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (search_tx, mut search_rx) = tokio::sync::mpsc::channel(8);
    let (interrupt, _) = broadcast::channel(8);
    let search_results = Arc::new(Mutex::new(vec![]));
    let access_token = config::access_token();

    if let Err(err) = &access_token {
        error!("Failed to get config file: {err}");
    };

    let mut plugin = StackOverflowPlugin {
        items: Arc::clone(&search_results),
        search_tx,
        interrupt,
        access_token: access_token.unwrap(),
    };

    let _ = tokio::join!(
        plugin.run(),
        dispatch_search_result(&mut search_rx, search_results)
    );
}

#[cfg(test)]
mod test {
    use crate::search_posts;
    use speculoos::prelude::*;
    use tokio_test::block_on;

    #[test]
    fn should_get_posts_from_stackoverflow() -> anyhow::Result<()> {
        let access_token = crate::config::access_token()?;
        let posts = block_on(search_posts("spring boot", &access_token));

        assert_that!(posts)
            .is_ok()
            .map(|response| &response.items)
            .has_length(8);

        Ok(())
    }
}
