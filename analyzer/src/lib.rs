use std::{path::PathBuf, str::FromStr};

use graph::Graph;
use lsp::populate_symbols;
use lsp_client::lsp_types::{
    ClientCapabilities, InitializeParams, InitializedParams, Uri, WindowClientCapabilities,
};

mod file_structure;
mod lsp;

use file_structure::populate_file_structure;

pub struct Analyzer {
    path: PathBuf,
    lsp_client: lsp_client::LspClient,
}

impl Analyzer {
    pub async fn start() -> Result<Self, anyhow::Error> {
        let lsp_client = lsp_client::LspClient::start("rust-analyzer")?;
        let path = std::env::current_dir()?;
        let path_uri = Uri::from_str(&format!("file://{}", path.to_str().unwrap()))?;
        let name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Failed to get current directory name"))?
            .to_string_lossy()
            .to_string();
        let params = InitializeParams {
            workspace_folders: Some(vec![lsp_client::lsp_types::WorkspaceFolder {
                uri: path_uri,
                name,
            }]),
            capabilities: ClientCapabilities {
                window: Some(WindowClientCapabilities {
                    work_done_progress: Some(true),
                    ..Default::default()
                }),
                text_document: Some(lsp_client::lsp_types::TextDocumentClientCapabilities {
                    document_symbol: Some(
                        lsp_client::lsp_types::DocumentSymbolClientCapabilities {
                            hierarchical_document_symbol_support: Some(true),
                            ..Default::default()
                        },
                    ),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        lsp_client
            .initialize(params, |_| InitializedParams {})
            .await?;
        lsp_client.wait_for_indexing_to_complete().await?;

        Ok(Self { lsp_client, path })
    }

    pub async fn stop(self) -> Result<(), anyhow::Error> {
        self.lsp_client.shutdown().await?;
        self.lsp_client.exit().await?;
        Ok(())
    }

    pub async fn graph(&self) -> Result<Graph, anyhow::Error> {
        let mut graph = Graph::default();

        populate_file_structure(&mut graph, &self.path)?;
        populate_symbols(&mut graph, &self.lsp_client).await?;

        std::fs::write("graph.json", serde_json::to_string_pretty(&graph).unwrap()).unwrap();

        Ok(graph)
    }
}
