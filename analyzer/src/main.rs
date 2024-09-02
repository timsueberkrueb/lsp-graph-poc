use analyzer::Analyzer;

#[tokio::main]
async fn main() {
    let a = Analyzer::start().await.unwrap();
    a.graph().await.unwrap();
    a.stop().await.unwrap();
}
