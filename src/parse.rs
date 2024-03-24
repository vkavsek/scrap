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

// TODO: TEST with irrelevant divs inserted and just pages with entries
// FIXME:
async fn parse_html(html: Arc<String>) -> Result<Vec<(String, String)>> {
    let words_vec = spawn_blocking({
        let html = html.clone();
        move || {
            let doc = Html::parse_document(&html);

            // TODO: HANDLE ERROR:
            let entry_selector = Selector::parse(r#"div[class="list-group-item entry"]"#).unwrap();
            let entry_divs = doc.select(&entry_selector);

            // There shouldn't be more than 30 entries per page.
            let mut words_vec = Vec::with_capacity(30);
            for entry_div in entry_divs {
                let span_selector = Selector::parse("span").unwrap();
                let anchor_selector = Selector::parse("a").unwrap();

                let div = Html::parse_fragment(&entry_div.html());

                let token = div
                    .select(&anchor_selector)
                    .next()
                    .expect("Entry didn't contain an anchor")
                    .inner_html();

                // TODO: HANDLE ERROR:
                let qualifier_selector =
                    Selector::parse(r#"span[data-group="header qualifier"]"#).unwrap();
                let mut qual_text = String::new();
                if let Some(qualifier) = div.select(&qualifier_selector).next() {
                    qual_text = Html::parse_fragment(&qualifier.inner_html())
                        .select(&span_selector)
                        .next()
                        .expect("Qualifier span didn't contain another span!")
                        .inner_html();
                }

                let word_entry = format!("{token:<20}{qual_text:<20}\n");
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
}
