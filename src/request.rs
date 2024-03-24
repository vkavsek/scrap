use anyhow::Result;
use chrono::Local;
use reqwest::Client;
use tokio::task::JoinSet;

use crate::{info_time, PAGES_PER_BLOCK, START_PAGE};

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

async fn request_page_html(client: Client, page_num: usize) -> Result<String> {
    // TODO: Errors ?
    let res = client
        .get(format!(
            "https://fran.si/iskanje?page={page_num}&FilteredDictionaryIds=133&View=1&Query=*"
        ))
        .send()
        .await
        .unwrap();
    let html = res.text().await.unwrap();
    Ok(html)
}
