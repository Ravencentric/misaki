# misaki

misaki is a fast, asynchronous link checker with optional [FlareSolverr](https://github.com/FlareSolverr/FlareSolverr) support.

This repository contains two crates:

- `misaki-core`: The core library that provides the link-checking functionality.
- `misaki-cli`: A command-line interface for `misaki-core`.

## misaki-core

`misaki-core` is a library for checking the status of URLs asynchronously.

### Usage

Add `misaki-core` to your `Cargo.toml`:

```console
$ cargo add misaki-core
```

Here is an example of how to use `misaki-core`:

```rust
use anyhow::Result;
use futures::StreamExt;
use misaki_core::LinkChecker;

#[tokio::main]
async fn main() -> Result<()> {
    let urls = vec!["https://httpbin.org/status/200"; 10];
    let checker = LinkChecker::builder().build().await?;
    {
        let iter = checker.check_all(urls).await;
        let mut iter = std::pin::pin!(iter);

        while let Some(status) = iter.next().await {
            println!("{:?}", status);
        }
    }
    checker.close().await?;
    Ok(())
}
```

## misaki-cli

`misaki-cli` is a command-line tool for checking links.

### Installation

You can install `misaki-cli` from crates.io using cargo:

```bash
cargo install misaki-cli
```

Alternatively, pre-built binaries for various platforms are available on the [GitHub releases page](https://github.com/Ravencentric/misaki/releases/latest).

### Usage

You can pipe a list of URLs to `misaki` to check them:

```bash
cat urls.txt | misaki -
```

Or, you can pass a single URL directly as an argument:

```bash
misaki https://google.com
```

#### With FlareSolverr

To use FlareSolverr, provide the base URL of your FlareSolverr instance using the `--flaresolverr` flag.

```bash
cat urls.txt | misaki - --flaresolverr http://localhost:8191
```

## License

This project is licensed under either of

- Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
