/*
Copyright 2024 The openBCE Authors.
Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at
    http://www.apache.org/licenses/LICENSE-2.0
Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

mod cfg;
mod cmd;

use std::error::Error;

use clap::Parser;
use tracing_subscriber::{filter::EnvFilter, filter::LevelFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Log level is set from, in order of preference:
    // 1. RUST_LOG environment variable
    // 2. Level::Info
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
        .add_directive("tower=warn".parse()?)
        .add_directive("rustls=warn".parse()?)
        .add_directive("h2=warn".parse()?);

    tracing_subscriber::registry()
        .with(fmt::Layer::default().compact().with_writer(std::io::stderr))
        .with(env_filter)
        .try_init()?;

    let options = cfg::ChariotOptions::parse();

    match options.command {
        cfg::Commands::Runp => {
            cmd::runp::run().await?;
        }
        cfg::Commands::Runc { file } => {
            cmd::runc::run(file).await?;
        }
    }

    Ok(())
}
