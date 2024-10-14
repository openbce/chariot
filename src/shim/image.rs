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

use std::collections::HashMap;

use stdng::{logs::TraceFn, trace_fn};
use tokio::net::UnixStream;
use tonic::transport::Channel;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use tracing::{debug, info};

use self::crirpc::image_service_client::ImageServiceClient;
use self::crirpc::{
    ImageFsInfoRequest, ImageFsInfoResponse, ImageStatusRequest, ImageStatusResponse,
    ListImagesRequest, ListImagesResponse, PullImageRequest, PullImageResponse, RemoveImageRequest,
    RemoveImageResponse,
};
use crate::cfg::ChariotOptions;

use chariot::apis::ChariotError;
use chariot::cri::{self as crirpc, Image, ImageSpec};

pub struct ImageShim {
    clients: HashMap<String, ImageServiceClient<Channel>>,
    default_runtime: String,
}

impl ImageShim {
    pub async fn connect(opts: ChariotOptions) -> Result<Self, ChariotError> {
        let mut clients = HashMap::new();

        for rt in opts.runtimes {
            let uri = rt.endpoint.find("://").unwrap_or(rt.endpoint.len());
            let (protocol, path) = rt.endpoint.split_at(uri + 3);
            let host_path = path.to_string();
            let mut client = match protocol.to_lowercase().as_str() {
                "unix://" => {
                    // let host_path = path.clone();
                    let channel = Endpoint::try_from("http://[::]:50051")
                        .map_err(|e| ChariotError::Network(e.to_string()))?
                        .connect_with_connector(service_fn(move |_: Uri| {
                            UnixStream::connect(host_path.clone())
                        }))
                        .await
                        .map_err(|e| ChariotError::Network(e.to_string()))?;

                    ImageServiceClient::new(channel)
                }
                _ => ImageServiceClient::connect(rt.endpoint.clone())
                    .await
                    .map_err(|e| ChariotError::Network(e.to_string()))?,
            };
            // Log image FS info
            let resp = client
                .image_fs_info(ImageFsInfoRequest {})
                .await
                .map_err(|e| ChariotError::Cri(e.to_string()))?;
            let fs_info = resp.into_inner();

            for fs in fs_info.container_filesystems {
                info!(
                    "{} container FS: {:?}",
                    rt.name,
                    fs.fs_id.map(|i| i.mountpoint),
                );
            }

            for fs in fs_info.image_filesystems {
                info!("{} image FS: {:?}", rt.name, fs.fs_id.map(|i| i.mountpoint),);
            }

            clients.insert(rt.name, client);
        }

        Ok(ImageShim {
            clients,
            default_runtime: opts.default,
        })
    }
}

impl ImageShim {
    fn get_default_client(&self) -> Result<ImageServiceClient<Channel>, tonic::Status> {
        self.get_client(&self.default_runtime)
    }

    fn get_client(&self, name: &str) -> Result<ImageServiceClient<Channel>, tonic::Status> {
        let name = match name.is_empty() {
            true => self.default_runtime.as_str(),
            false => name,
        };

        let client = self.clients.get(name);
        match client {
            Some(c) => Ok(c.clone()),
            None => Err(tonic::Status::not_found(format!(
                "no connection for {}",
                name
            ))),
        }
    }
}

#[async_trait::async_trait]
impl crirpc::image_service_server::ImageService for ImageShim {
    async fn list_images(
        &self,
        request: tonic::Request<ListImagesRequest>,
    ) -> Result<tonic::Response<ListImagesResponse>, tonic::Status> {
        trace_fn!("ImageShim::list_images");

        let req = request.into_inner();
        debug!("list_images request: {:?}", req);

        let mut resp = ListImagesResponse { images: vec![] };

        for (name, client) in &self.clients {
            let mut client = client.clone();
            let imgs = client
                .list_images(tonic::Request::new(req.clone()))
                .await?
                .into_inner();

            resp.images.extend(
                imgs.images
                    .iter()
                    .map(|i| {
                        let mut img = i.clone();

                        if let Some(mut s) = img.spec {
                            s.runtime_handler = name.clone();
                            img.spec = Some(s);
                        }

                        img
                    })
                    .collect::<Vec<_>>(),
            );
        }

        debug!("list_images response: {:?}", resp);

        Ok(tonic::Response::new(resp))
    }

    async fn image_status(
        &self,
        request: tonic::Request<ImageStatusRequest>,
    ) -> Result<tonic::Response<ImageStatusResponse>, tonic::Status> {
        trace_fn!("ImageShim::image_status");

        let req = request.into_inner();
        debug!("image_status request: {:?}", req);

        match req.image {
            Some(ref img) => {
                let mut client = self.get_client(&img.runtime_handler)?;
                let mut resp = client
                    .image_status(tonic::Request::new(req.clone()))
                    .await?
                    .into_inner();

                let handler = img.runtime_handler.clone();
                resp.image = resp.image.map(|img| {
                    let spec = img.spec.map(|s| ImageSpec {
                        runtime_handler: handler,
                        ..s
                    });

                    Image { spec, ..img }
                });

                Ok(tonic::Response::new(resp))
            }
            None => Err(tonic::Status::not_found("no image info in request")),
        }
    }

    /// PullImage pulls an image with authentication config.
    async fn pull_image(
        &self,
        request: tonic::Request<PullImageRequest>,
    ) -> Result<tonic::Response<PullImageResponse>, tonic::Status> {
        trace_fn!("ImageShim::pull_image");
        let req = request.into_inner();
        debug!("pull_image request: {:?}", req);

        match req.image {
            Some(ref img) => {
                let mut client = self.get_client(&img.runtime_handler)?;
                client.pull_image(tonic::Request::new(req.clone())).await
            }
            None => Err(tonic::Status::not_found("no image info in request")),
        }
    }

    async fn remove_image(
        &self,
        request: tonic::Request<RemoveImageRequest>,
    ) -> Result<tonic::Response<RemoveImageResponse>, tonic::Status> {
        trace_fn!("ImageShim::remove_image");
        let req = request.into_inner();
        debug!("remove_image request: {:?}", req);

        match req.image {
            Some(ref img) => {
                let mut client = self.get_client(&img.runtime_handler)?;
                client.remove_image(tonic::Request::new(req.clone())).await
            }
            None => Err(tonic::Status::not_found("no image info in request")),
        }
    }

    /// ImageFSInfo returns information of the filesystem that is used to store images.
    async fn image_fs_info(
        &self,
        request: tonic::Request<ImageFsInfoRequest>,
    ) -> Result<tonic::Response<ImageFsInfoResponse>, tonic::Status> {
        trace_fn!("ImageShim::image_fs_info");

        // TODO (k82cn): merge image fs info into single response
        let mut client = self.get_default_client()?;
        client.image_fs_info(request).await
    }
}
