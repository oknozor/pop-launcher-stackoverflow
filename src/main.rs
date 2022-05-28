use anyhow::Result;
use pop_launcher_toolkit::launcher::{Indice, PluginResponse, PluginSearchResult};
use pop_launcher_toolkit::plugin_trait::tracing::error;
use pop_launcher_toolkit::plugin_trait::{async_trait, PluginExt};
use pop_launcher_toolkit::plugins::xdg_open;
use serde::Deserialize;

#[derive(Default)]
struct StackOverflowPlugin {
    items: Vec<StackOverFlowPost>,
}

#[async_trait]
impl PluginExt for StackOverflowPlugin {
    fn name(&self) -> &str {
        "stackoverflow"
    }

    async fn search(&mut self, query: &str) {
        match query.strip_prefix("stk ") {
            Some(query) if !query.is_empty() => match search_posts(query) {
                Ok(response) => {
                    self.items = response.items;
                    for (idx, post) in self.items.iter().enumerate() {
                        self.respond_with(PluginResponse::Append(PluginSearchResult {
                            id: idx as u32,
                            name: post.title.clone(),
                            description: post.link.clone(),
                            ..Default::default()
                        }))
                        .await
                    }
                }
                Err(err) => error!("Failed to get posts from stackoverflow: {err}"),
            },
            _ => {}
        }

        self.respond_with(PluginResponse::Finished).await
    }

    async fn activate(&mut self, id: Indice) {
        match self.items.get(id as usize) {
            Some(post) => xdg_open(&post.link),
            None => error!("Failed to get post at index {id}"),
        }

        self.respond_with(PluginResponse::Close).await;
    }
}

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
