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

use crate::rpc::cri::image_service_client::ImageServiceClient;
use crate::rpc::cri::image_service_server::ImageServiceServer;
use crate::rpc::cri::runtime_service_client::RuntimeServiceClient;
use crate::rpc::cri::runtime_service_server::RuntimeServiceServer;

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // construct a subscriber that prints formatted traces to stdout
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    // use that subscriber to process traces emitted after this point
    tracing::subscriber::set_global_default(subscriber)?;

    let args = cfg::Options::parse();

    let image_client= ImageServiceClient::connect(args.xpu_address.clone()).await?;
    let runtime_client = RuntimeServiceClient::connect(args.xpu_address.clone()).await?;

    let image_svc = cri::image::ImageShim {
        xpu_client: image_client,
    };
    let runtime_svc = cri::runtime::RuntimeShim {
        xpu_client: runtime_client,
    };

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
