fn main() {
    if cfg!(feature = "server") {
        server::main()
    }
}
