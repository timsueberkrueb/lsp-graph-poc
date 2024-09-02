use std::ffi::OsStr;

use anyhow::anyhow;
use jsonrpsee::core::client::Client;
use jsonrpsee::core::client::ClientBuilder;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::traits::ToRpcParams;
use lsp_types::{
    InitializeParams, InitializeResult, InitializedParams, WorkspaceSymbolParams,
    WorkspaceSymbolResponse,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::process;

pub use lsp_types;

pub mod progress;
mod transport;

pub struct LspClient {
    /// The LSP server process.
    #[allow(dead_code)]
    child: process::Child,
    /// JSONRPC connection to the LSP server.
    jsonrpc_client: Client,
}

impl LspClient {
    /// Start an LSP server and returns a client for interacting with it.
    pub fn start<S: AsRef<OsStr>>(program: S) -> Result<Self, anyhow::Error> {
        let program = program.as_ref().to_owned();
        let mut command = process::Command::new(&program);
        command
            .stdout(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped());
        let mut child = command.spawn()?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to acquire child stdout"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to acquire child stdin"))?;

        let sender = transport::StdioSender::new(stdin);
        let receiver = transport::StdioReceiver::new(stdout);

        let jsonrpc_client = ClientBuilder::default().build_with_tokio(sender, receiver);

        Ok(Self {
            child,
            jsonrpc_client,
        })
    }

    pub async fn initialize<F: FnOnce(InitializeResult) -> InitializedParams>(
        &self,
        params: InitializeParams,
        on_initialized: F,
    ) -> Result<(), anyhow::Error> {
        let result: InitializeResult = self.request("initialize", params).await?;
        let initialized_params = on_initialized(result);
        self.notify("initialized", initialized_params).await?;
        Ok(())
    }

    pub async fn wait_for_indexing_to_complete(&self) -> Result<(), anyhow::Error> {
        progress::wait_for_indexing_to_complete(&self.jsonrpc_client).await
    }

    pub async fn workspace_symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<WorkspaceSymbolResponse, anyhow::Error> {
        self.request("workspace/symbol", params).await
    }

    pub async fn workspace_symbol_resolve(
        &self,
        params: lsp_types::WorkspaceSymbol,
    ) -> Result<lsp_types::WorkspaceSymbol, anyhow::Error> {
        self.request("workspaceSymbol/resolve", params).await
    }

    pub async fn document_symbol(
        &self,
        params: lsp_types::DocumentSymbolParams,
    ) -> Result<lsp_types::DocumentSymbolResponse, anyhow::Error> {
        self.request("textDocument/documentSymbol", params).await
    }

    pub async fn text_document_moniker(
        &self,
        params: lsp_types::TextDocumentPositionParams,
    ) -> Result<lsp_types::Moniker, anyhow::Error> {
        self.request("textDocument/moniker", params).await
    }

    pub async fn did_open(
        &self,
        params: lsp_types::DidOpenTextDocumentParams,
    ) -> Result<(), anyhow::Error> {
        self.notify("textDocument/didOpen", params).await
    }

    pub async fn shutdown(&self) -> Result<(), anyhow::Error> {
        self.request("shutdown", serde_json::Value::Null).await
    }

    pub async fn exit(&self) -> Result<(), anyhow::Error> {
        self.notify("exit", ()).await
    }

    async fn request<T: Serialize + Send, R: DeserializeOwned>(
        &self,
        method: &str,
        params: T,
    ) -> Result<R, anyhow::Error> {
        let response = self
            .jsonrpc_client
            .request(method, RpcParam(params))
            .await?;
        Ok(response)
    }

    async fn notify<T: Serialize + Send>(
        &self,
        method: &str,
        params: T,
    ) -> Result<(), anyhow::Error> {
        self.jsonrpc_client
            .notification(method, RpcParam(params))
            .await?;
        Ok(())
    }
}

/// Wrapper type for a single RPC parameter.
struct RpcParam<S: serde::Serialize + Send>(S);

impl<S: serde::Serialize + Send> ToRpcParams for RpcParam<S> {
    fn to_rpc_params(self) -> Result<Option<Box<serde_json::value::RawValue>>, serde_json::Error> {
        let raw_value = serde_json::value::to_raw_value(&self.0)?;
        Ok(Some(raw_value))
    }
}
