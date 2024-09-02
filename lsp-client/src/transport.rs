use std::sync::Arc;
use thiserror::Error;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::Mutex;

use jsonrpsee::core::{
    async_trait,
    client::{ReceivedMessage, TransportReceiverT, TransportSenderT},
};

#[derive(Debug, Error)]
pub enum StdioTransportError {
    #[error("I/O Error: {0}")]
    Io(#[from] io::Error),
    #[error("Serialization Error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Parse Int Error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("FromUtf8 Error: {0}")]
    FromUtf8(#[from] std::string::FromUtf8Error),
}

pub struct StdioSender {
    sender: Arc<Mutex<ChildStdin>>,
}

impl StdioSender {
    pub fn new(stdin: ChildStdin) -> Self {
        Self {
            sender: Arc::new(Mutex::new(stdin)),
        }
    }
}

#[async_trait]
impl TransportSenderT for StdioSender {
    type Error = StdioTransportError;

    async fn send(&mut self, msg: String) -> Result<(), Self::Error> {
        let mut writer = self.sender.lock().await;
        let header = format!("Content-Length: {}\r\n\r\n", msg.len());
        writer.write_all(header.as_bytes()).await?;
        writer.write_all(msg.as_bytes()).await?;
        writer.flush().await?;
        Ok(())
    }

    // Optionally override send_ping and close methods if needed.
}

pub struct StdioReceiver {
    reader: Arc<Mutex<BufReader<ChildStdout>>>,
}

impl StdioReceiver {
    pub fn new(stdout: ChildStdout) -> Self {
        Self {
            reader: Arc::new(Mutex::new(BufReader::new(stdout))),
        }
    }
}

#[async_trait]
impl TransportReceiverT for StdioReceiver {
    type Error = StdioTransportError;

    async fn receive(&mut self) -> Result<ReceivedMessage, Self::Error> {
        let mut reader = self.reader.lock().await;
        let mut buf = String::new();
        let mut content_length = None;

        // Read headers
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            if line == "\r\n" {
                break;
            }
            if line.starts_with("Content-Length:") {
                let len: usize = line
                    .trim_start_matches("Content-Length:")
                    .trim()
                    .parse()
                    .map_err(StdioTransportError::from)?;
                content_length = Some(len);
            }
        }

        // Read message body
        if let Some(len) = content_length {
            let mut body = vec![0; len];
            reader.read_exact(&mut body).await?;
            buf = String::from_utf8(body).map_err(StdioTransportError::from)?;
        }

        Ok(ReceivedMessage::Text(buf))
    }
}
