use container::Container;
use runtime::ContainerRuntime;

mod container;
mod runtime;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let wasm_module_path = std::env::args()
        .nth(1)
        .expect("missing wasm module path in first argument");

    let path = wasm_module_path.clone();
    let runtime = ContainerRuntime::new()?;
    let mut container = Container::load_from_file(&runtime, path).await?;

    let mut output = container.output.take().expect("output channel missing");
    tokio::spawn(async move {
        while let Some(message) = output.recv().await {
            println!("output: {}", message);
        }
    });

    container.init().await?;

    Ok(())
}
