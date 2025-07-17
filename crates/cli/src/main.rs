use anyhow::Result;
use clap::Parser;
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap_stdin::MaybeStdin;
use futures::StreamExt;
use itertools::Itertools;
use misaki_core::{LinkChecker, UrlStatus};
use owo_colors::OwoColorize;
use std::{
    io::{self, Write},
    time::Instant,
};

// Stolen from https://github.com/astral-sh/uv/blob/96cfca1c8fe711d24215f9d1bb91cea7002aa087/crates/uv-cli/src/lib.rs#L69-L74
const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

/// Fast, asynchronous link checker with optional FlareSolverr support.
#[derive(Parser)]
#[command(name ="misaki", version, about, long_about = None, styles = STYLES)]
struct Cli {
    /// URL(s) to check. Can be a single URL or multiple, separated by newlines.
    url: MaybeStdin<String>,
    /// Base URL for a FlareSolverr instance (e.g., http://localhost:8191).
    #[arg(long)]
    flaresolverr: Option<String>,
    /// Output results in JSON format.
    #[arg(long)]
    json: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let checker = LinkChecker::builder()
        .maybe_flaresolverr(cli.flaresolverr)
        .build()
        .await?;

    let cleaner = checker.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        let _ = cleaner.close().await;
        std::process::exit(1);
    });

    let urls = cli.url.lines().map(str::trim).unique();
    {
        let iter = checker.check_all(urls).await;
        let mut iter = std::pin::pin!(iter);
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        if cli.json {
            while let Some(status) = iter.next().await {
                writeln!(handle, "{}", serde_json::to_string(&status)?)?;
            }
        } else {
            let start_time = Instant::now();
            let mut total = 0;
            let mut success = 0;
            let mut fail = 0;
            let mut unknown = 0;

            while let Some(status) = iter.next().await {
                total += 1;
                let UrlStatus { url, status } = status;
                match status {
                    Some(200) => {
                        success += 1;
                        writeln!(handle, "[{}] {}", 200.green(), url)?
                    }
                    Some(code) => {
                        fail += 1;
                        writeln!(handle, "[{}] {}", code.red(), url)?
                    }
                    None => {
                        unknown += 1;
                        writeln!(handle, "[{}] {}", "Unknown".yellow(), url)?
                    }
                };
            }
            let elapsed = start_time.elapsed();
            writeln!(
                handle,
                "\nChecked {} URLs in {:.2?}: {} successful, {} failed, {} unknown.",
                total,
                elapsed,
                success.green(),
                fail.red(),
                unknown.yellow()
            )?;
        }
    }
    checker.close().await?;
    Ok(())
}
