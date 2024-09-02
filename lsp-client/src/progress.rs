use std::collections::HashSet;

use jsonrpsee::core::client::Client;
use jsonrpsee::core::client::Subscription;
use jsonrpsee::core::client::SubscriptionClientT;
use lsp_types::NumberOrString;
use lsp_types::ProgressParams;

pub async fn wait_for_indexing_to_complete(client: &Client) -> Result<(), anyhow::Error> {
    let mut waiting_for =
        HashSet::from([NumberOrString::String("rustAnalyzer/Indexing".to_owned())]);

    // Subscribe to notifications
    let mut subscription: Subscription<ProgressParams> = client
        .subscribe_to_method("$/progress")
        .await
        .expect("Failed to subscribe to progress notifications");

    while let Some(notification) = subscription.next().await.transpose()? {
        let ProgressParams { token, value } = notification;
        let lsp_types::ProgressParamsValue::WorkDone(progress) = value;

        match progress {
            lsp_types::WorkDoneProgress::Begin(_) => {
                waiting_for.insert(token.clone());
            }
            lsp_types::WorkDoneProgress::Report(_) => {}
            lsp_types::WorkDoneProgress::End(_) => {
                waiting_for.remove(&token);
            }
        }

        if waiting_for.is_empty() {
            break;
        }
    }

    Ok(())
}
