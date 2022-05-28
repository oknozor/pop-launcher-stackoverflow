use anyhow::Result;
use pop_launcher_toolkit::launcher::{Indice, PluginResponse, PluginSearchResult};
use pop_launcher_toolkit::plugin_trait::tracing::error;
use pop_launcher_toolkit::plugin_trait::{async_trait, PluginExt};
use pop_launcher_toolkit::plugins::xdg_open;
use serde::Deserialize;

// Our Plugin, holding the Stackoverflow search results
#[derive(Default)]
struct StackOverflowPlugin {
    items: Vec<StackOverFlowPost>,
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
        // Get rid of our plugin prefix
        match query.strip_prefix("stk ") {
            Some(query) if !query.is_empty() => match search_posts(query) {
                // Store the search results
                Ok(response) => self.items = response.items,
                // If anything goes wrong during the HTTP call lets write that to the
                // logfile at $HOME/.state/pop-launcher/stackoverflow.log
                Err(err) => error!("Failed to get posts from stackoverflow: {err}"),
            },
            _ => {}
        }

        // Send our stored search results to pop-launcher
        for (idx, post) in self.items.iter().enumerate() {
            self.respond_with(PluginResponse::Append(PluginSearchResult {
                id: idx as u32,
                name: post.title.clone(),
                description: post.link.clone(),
                ..Default::default()
            }))
            .await
        }

        // Tell pop-launcher we are done and the results are ready to be displayed
        self.respond_with(PluginResponse::Finished).await
    }

    // This is called whenever the user ask to activate the current selected item
    async fn activate(&mut self, id: Indice) {
        // Get our stackoverflow post at the selected index
        // and open it in the browser with the `xdg_open` helper function.
        match self.items.get(id as usize) {
            Some(post) => xdg_open(&post.link),
            None => error!("Failed to get post at index {id}"),
        }

        // We are done, tell pop-launcher to close the client
        self.respond_with(PluginResponse::Close).await;
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
}

// Call the stackoverflow api with the provided `intitle` query param
fn search_posts(intitle: &str) -> Result<StackOverFlowResponse> {
    let response = ureq::get("https://api.stackexchange.com/2.3/search?")
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
    let mut plugin = StackOverflowPlugin::default();
    plugin.run().await
}

#[cfg(test)]
mod test {
    use crate::search_posts;
    use speculoos::prelude::*;

    #[test]
    fn should_get_posts_from_stackoverflow() {
        let posts = search_posts("spring boot");

        assert_that!(posts)
            .is_ok()
            .map(|response| &response.items)
            .has_length(8);
    }
}
