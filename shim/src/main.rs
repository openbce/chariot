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
mod common;
mod cri;
mod rpc;

use std::error::Error;
use std::fs;
use std::path::Path;

use clap::Parser;
#[cfg(unix)]
use tokio::net::UnixListener;
#[cfg(unix)]
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::{filter::EnvFilter, filter::LevelFilter, fmt, prelude::*};

use crate::cri::image::ImageShim;
use crate::cri::runtime::RuntimeShim;
use crate::rpc::cri::image_service_server::ImageServiceServer;
use crate::rpc::cri::runtime_service_server::RuntimeServiceServer;

#[cfg(unix)]
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

    let args = cfg::Options::parse();

    info!(
        "Chariot start the CRI listener at {}",
        cri::DEFAULT_UNIX_SOCKET
    );
    info!("Connecting to Host CRI at {}", args.host_cri.clone());
    info!("Connecting to XPU CRI at {}", args.xpu_cri.clone());

    let image_svc = ImageShim::connect(args.host_cri.clone(), args.xpu_cri.clone()).await?;
    let runtime_svc = RuntimeShim::connect(args.host_cri.clone(), args.xpu_cri.clone()).await?;

    // TODO(k82cn): use the address from args.
    fs::create_dir_all(cri::DEFAULT_UNIX_SOCKET_DIR)?;

    if Path::new(cri::DEFAULT_UNIX_SOCKET).exists() {
        fs::remove_file(cri::DEFAULT_UNIX_SOCKET)?;
    }

    let uds = UnixListener::bind(cri::DEFAULT_UNIX_SOCKET)?;
    let uds_stream = UnixListenerStream::new(uds);

    Server::builder()
        .add_service(RuntimeServiceServer::new(runtime_svc))
        .add_service(ImageServiceServer::new(image_svc))
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}
