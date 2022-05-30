// #[cfg(feature = "server")]
// mod server;

fn main() {
    if cfg!(feature = "server") {
        server::main()
    }
}
