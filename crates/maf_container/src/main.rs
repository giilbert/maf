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

    init_one_container(&runtime, &path, 1).await?;
    // let two = init_one_container(&runtime, &path, 2);

    // tokio::try_join!(one, two)?;

    Ok(())
}

async fn init_one_container(
    runtime: &ContainerRuntime,
    path: &str,
    number: u32,
) -> anyhow::Result<()> {
    let mut container = Container::load_from_file(&runtime, path).await?;

    let mut output = container.output.take().expect("output channel missing");

    let handle = tokio::spawn(async move {
        while let Some(message) = output.recv().await {
            println!("output({number}): {message}");
        }

        println!("output done");
    });

    match container.init().await {
        Ok(_) => {
            println!("container initialized");
        }
        Err(e) => {
            println!("container failed to initialize: {e}");
        }
    };

    handle.await?;

    Ok(())
}
