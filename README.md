# misaki

misaki is a fast, asynchronous link checker with optional FlareSolverr support, written in Rust.

## Features

- Fast, asynchronous link checking
- Optional [FlareSolverr](https://github.com/FlareSolverr/FlareSolverr) support to bypass Cloudflare
- JSON output

## Installation

You can install Misaki from crates.io using cargo:

```bash
cargo install misaki-cli
```

## Usage

You can pipe a list of URLs to `misaki` to check them:

```bash
cat urls.txt | misaki -
```

Or, you can pass a single URL directly as an argument:

```bash
misaki https://google.com
```

### With FlareSolverr

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
