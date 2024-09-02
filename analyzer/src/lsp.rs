use std::path::{Path, PathBuf};
use std::str::FromStr;

use lsp_client::lsp_types;
use lsp_client::{lsp_types::Uri, LspClient};

use graph::{EdgeData, Graph, NodeContents, NodeData, NodeId};

pub async fn populate_symbols(
    graph: &mut Graph,
    lsp_client: &LspClient,
) -> Result<(), anyhow::Error> {
    let nodes: Vec<_> = graph.nodes().collect();
    for node_id in nodes {
        let node = graph.node(node_id).unwrap();
        let graph::NodeContents::File { path, .. } = &node.contents else {
            continue;
        };
        let path = path.to_str().unwrap();
        let path = PathBuf::from(path);
        let Some(ext) = path.extension() else {
            continue;
        };
        if ext == "rs" {
            populate_document_symbols(&path, node_id, graph, lsp_client).await?;
        }
    }
    Ok(())
}

pub async fn populate_document_symbols(
    path: &Path,
    node_id: NodeId,
    graph: &mut Graph,
    lsp_client: &LspClient,
) -> Result<(), anyhow::Error> {
    let document_symbols = retrieve_document_symbols(path, lsp_client).await?;
    add_document_symbols(graph, node_id, document_symbols)?;

    Ok(())
}

async fn retrieve_document_symbols(
    path: &Path,
    lsp_client: &LspClient,
) -> Result<lsp_types::DocumentSymbolResponse, anyhow::Error> {
    let uri = Uri::from_str(&format!(
        "file://{}",
        path.to_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to convert path to string"))?
    ))?;

    lsp_client
        .document_symbol(lsp_client::lsp_types::DocumentSymbolParams {
            text_document: lsp_client::lsp_types::TextDocumentIdentifier::new(uri),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
}

fn add_document_symbols(
    graph: &mut Graph,
    file_id: NodeId,
    document_symbols: lsp_types::DocumentSymbolResponse,
) -> Result<(), anyhow::Error> {
    let lsp_types::DocumentSymbolResponse::Nested(symbols) = document_symbols else {
        anyhow::bail!("Flat document symbols are not supported yet");
    };

    for symbol in symbols {
        add_document_symbol(graph, file_id, symbol)?;
    }

    Ok(())
}

fn add_document_symbol(
    graph: &mut Graph,
    parent_id: NodeId,
    symbol: lsp_types::DocumentSymbol,
) -> Result<(), anyhow::Error> {
    let contents = NodeContents::Item {
        display_name: symbol.name,
        moniker: None,
    };
    let node = NodeData { contents };
    let item_id = graph.add_node(node);
    let edge = EdgeData {
        from: parent_id,
        to: item_id,
        relation: graph::Relation::IsParentOf,
    };
    graph.add_edge(edge);

    for child in symbol.children.unwrap_or_default() {
        add_document_symbol(graph, item_id, child)?;
    }

    Ok(())
}
