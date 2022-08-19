pub mod api;
pub mod ui;

#[tokio::main]
async fn main() {
    ui::init().await.unwrap();
}

