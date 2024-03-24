use std::sync::Arc;

use anyhow::Result;
use chrono::Local;
use scraper::{Html, Selector};
use tokio::{
    sync::{mpsc, oneshot::Sender},
    task::{spawn_blocking, JoinHandle, JoinSet},
};

use crate::info_time;

pub(crate) async fn parse_block(
    block_to_parse: JoinHandle<JoinSet<Result<String>>>,
    stop_tx: Option<Sender<()>>,
    str_tx: mpsc::Sender<Vec<(String, String)>>,
) -> Result<()> {
    let mut block_to_process = block_to_parse.await?;

    let mut found_empty_page = false;
    while let Some(task) = block_to_process.join_next().await {
        let html = task??;
        // Get all words + qualifiers
        let words = parse_html(html.into()).await?;
        if words.is_empty() {
            info_time!("found EMPTY page");
            found_empty_page = true;
            continue;
        }
        str_tx.send(words).await?;
    }

    if found_empty_page {
        if let Some(stop_send) = stop_tx {
            info_time!("sending STOP signal");
            let _ = stop_send.send(());
        }
    }
    Ok(())
}

// FIXME:
// TODO: TEST with irrelevant divs inserted and just pages with entries
async fn parse_html(html: Arc<String>) -> Result<Vec<(String, String)>> {
    let words_vec = spawn_blocking({
        let html = html.clone();
        move || {
            let doc = Html::parse_document(&html);

            // TODO: HANDLE ERROR:
            let entry_selector = Selector::parse(r#"div[class="entry"]"#).unwrap();
            let entry_divs = doc.select(&entry_selector);

            // There shouldn't be more than 30 entries per page.
            let mut words_vec = Vec::with_capacity(30);
            for entry_div in entry_divs {
                // TODO: HANDLE ERROR:
                let token_selector = Selector::parse(r#"div[class="token"]"#).unwrap();
                let qual_selector = Selector::parse(r#"div[class="qual"]"#).unwrap();

                let div = Html::parse_fragment(&entry_div.html());

                let token = div
                    .select(&token_selector)
                    .next()
                    .expect("Entry didn't contain a token")
                    .inner_html();

                let word_entry;
                if let Some(qualifier) = div.select(&qual_selector).next() {
                    let qual_text = qualifier.inner_html();
                    word_entry = format!("{:<20} | {:<20}\n", token.trim(), qual_text.trim());
                } else {
                    word_entry = format!("{}\n", token.trim());
                }

                words_vec.push((fmt_sortable_entry(&token), word_entry));
            }
            words_vec
        }
    })
    .await?;

    Ok(words_vec)
}

// TODO: Add regex + add other letters (r)
fn fmt_sortable_entry(token: &str) -> String {
    token
        .to_lowercase()
        .replace(['á', 'à', 'â', 'ã', 'ä'], "a")
        .replace(['é', 'è', 'ê', 'ë'], "e")
        .replace(['í', 'ì', 'î', 'ï'], "i")
        .replace(['ó', 'ò', 'ô', 'õ', 'ö'], "o")
        .replace(['ú', 'ù', 'û', 'ü'], "u")
        .replace(['ý', 'ÿ'], "y")
        .replace(['ñ'], "n")
        .replace(['ŕ'], "r")
}
