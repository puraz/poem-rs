fn main() -> anyhow::Result<()> {
    poem_rs::runtime::install_panic_hook();

    if let Err(error) = poem_rs::run() {
        poem_rs::runtime::append_launch_log(&format!("startup error: {error:#}"));
        return Err(error);
    }

    Ok(())
}
