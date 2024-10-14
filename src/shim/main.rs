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
mod runtime;
mod image;

use std::fs;
use std::path::Path;

use clap::Parser;
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::{filter::EnvFilter, filter::LevelFilter, fmt, prelude::*};

use crate::image::ImageShim;
use crate::runtime::RuntimeShim;
use chariot::cri::image_service_server::ImageServiceServer;
use chariot::cri::runtime_service_server::RuntimeServiceServer;
use chariot::apis::ChariotResult;

// The default Unix socket for Chariot shim.
pub const DEFAULT_UNIX_SOCKET_DIR: &str = "/run/chariot";
pub const DEFAULT_UNIX_SOCKET: &str = "/run/chariot/chariot.sock";


#[tokio::main]
async fn main() -> ChariotResult<()> {
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

    let args = cfg::Options::parse();

    let config = std::fs::read_to_string(args.config)?;
    let opts: cfg::ChariotOptions = toml::from_str(config.as_str())?;

    info!(
        "Chariot start the CRI listener at {}",
        DEFAULT_UNIX_SOCKET
    );

    let image_svc = ImageShim::connect(opts.clone()).await?;
    let runtime_svc = RuntimeShim::connect(opts.clone()).await?;

    // TODO(k82cn): use the address from args.
    fs::create_dir_all(DEFAULT_UNIX_SOCKET_DIR)?;

    if Path::new(DEFAULT_UNIX_SOCKET).exists() {
        fs::remove_file(DEFAULT_UNIX_SOCKET)?;
    }

    let uds = UnixListener::bind(DEFAULT_UNIX_SOCKET)?;
    let uds_stream = UnixListenerStream::new(uds);

    Server::builder()
        .add_service(RuntimeServiceServer::new(runtime_svc))
        .add_service(ImageServiceServer::new(image_svc))
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}
