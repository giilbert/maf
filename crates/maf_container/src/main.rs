use container::Container;
use runtime::ContainerRuntime;

mod container;
mod runtime;

fn main() -> anyhow::Result<()> {
    let wasm_module_path = std::env::args()
        .nth(1)
        .expect("missing wasm module path in first argument");

    let runtime = ContainerRuntime::new()?;
    let container = Container::load_from_file(&runtime, wasm_module_path)?;

    for line in container.output {
        println!("container: {}", line);
    }

    Ok(())
}
