use chrono::Local;
use reqwest::Client;
use tokio::task::JoinSet;

use crate::{info_time, Result, PAGES_PER_BLOCK, START_PAGE};

/// Returns a `JoinSet` of all the page requests in a block, so that they can be awaited.
pub(crate) async fn request_block(block_num: usize, client: Client) -> JoinSet<Result<String>> {
    info_time!("Requesting block: {block_num}");

    let mut task_set = JoinSet::new();

    let real_page_num = block_num * PAGES_PER_BLOCK + START_PAGE;
    for page_num in real_page_num..real_page_num + PAGES_PER_BLOCK {
        task_set.spawn({
            // Client uses Arc so we can clone cheaply
            let client = client.clone();

            async move { request_page_html(client, page_num).await }
        });
    }
    task_set
}

/// Requests a page and returns a `Result<String>` containing the HTML.
async fn request_page_html(client: Client, page_num: usize) -> Result<String> {
    let res = client
        .get(format!("http://127.0.0.1:3000/{page_num}"))
        .send()
        .await?;
    let html = res.text().await?;
    Ok(html)
}
