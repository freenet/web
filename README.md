# The [freenet.org](https://freenet.org/) website source code

## Directory overview

* [hugo-site](/hugo-site) - The main website generated using [Hugo](https://gohugo.io) static site generator.
* [rust/gkwasm](/rust/gkwasm) - The code that generates the webassembly used by [Ghost Keys](https://freenet.org/ghostkey/create/)
* [rust/gklib](/rust/gklib) - A Rust [crate](https://crates.io/crates/ghostkey_lib) for use with Ghost Keys
* [rust/cli](/rust/cli) - A Rust [command line tool](https://crates.io/crates/ghostkey) for use with Ghost Keys
* [rust/api](/rust/api) - The API used by the website to create new Ghost Keys, built using [Axom](https://crates.io/crates/axum)
