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
use crate::rpc::cri::{self as crirpc, Image, ImageSpec};

use crate::common::ChariotError;

pub struct ImageShim {
    pub xpu_client: ImageServiceClient<Channel>,
    pub host_client: ImageServiceClient<Channel>,
}

impl ImageShim {
    pub async fn connect(host_cri: String, xpu_cri: String) -> Result<Self, ChariotError> {
        let mut xpu_client = ImageServiceClient::connect(xpu_cri)
            .await
            .map_err(|e| ChariotError::NetworkError(e.to_string()))?;

        // Log XPU image FS info
        let resp = xpu_client
            .image_fs_info(ImageFsInfoRequest {})
            .await
            .map_err(|e| ChariotError::CriError(e.to_string()))?;
        let fs_info = resp.into_inner();

        for fs in fs_info.container_filesystems {
            info!("XPU container FS: {:?}", fs.fs_id.map(|i| i.mountpoint),);
        }

        for fs in fs_info.image_filesystems {
            info!("XPU image FS: {:?}", fs.fs_id.map(|i| i.mountpoint),);
        }

        let channel = Endpoint::try_from("http://[::]:50051")
            .map_err(|e| ChariotError::NetworkError(e.to_string()))?
            .connect_with_connector(service_fn(move |_: Uri| {
                let host_path = host_cri.clone();
                UnixStream::connect(host_path)
            }))
            .await
            .map_err(|e| ChariotError::NetworkError(e.to_string()))?;

        let mut host_client = ImageServiceClient::new(channel);

        // Log host image FS info
        let resp = host_client
            .image_fs_info(ImageFsInfoRequest {})
            .await
            .map_err(|e| ChariotError::CriError(e.to_string()))?;
        let fs_info = resp.into_inner();

        for fs in fs_info.container_filesystems {
            info!("Host container FS: {:?}", fs.fs_id.map(|i| i.mountpoint),);
        }

        for fs in fs_info.image_filesystems {
            info!("Host image FS: {:?}", fs.fs_id.map(|i| i.mountpoint),);
        }

        Ok(ImageShim {
            xpu_client,
            host_client,
        })
    }
}

#[async_trait::async_trait]
impl crirpc::image_service_server::ImageService for ImageShim {
    async fn list_images(
        &self,
        request: tonic::Request<ListImagesRequest>,
    ) -> Result<tonic::Response<ListImagesResponse>, tonic::Status> {
        trace_fn!("ImageShim::list_images");

        let mut host_client = self.host_client.clone();
        let mut xpu_client = self.xpu_client.clone();

        let req = request.into_inner();
        debug!("list_images request: {:?}", req);

        let host_imgs = host_client
            .list_images(tonic::Request::new(req.clone()))
            .await?
            .into_inner();
        let xpu_imgs = xpu_client
            .list_images(tonic::Request::new(req.clone()))
            .await?
            .into_inner();

        let mut resp = ListImagesResponse { images: vec![] };

        resp.images.extend(
            xpu_imgs
                .images
                .iter()
                .map(|i| {
                    let mut img = i.clone();

                    if let Some(mut s) = img.spec {
                        s.runtime_handler = "xpu".to_string();
                        img.spec = Some(s);
                    }

                    img
                })
                .collect::<Vec<_>>(),
        );

        resp.images.extend(host_imgs.images);

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

        if let Some(img) = req.image.clone() {
            if img.runtime_handler.as_str() == "xpu" {
                let mut client = self.xpu_client.clone();

                let mut resp = client
                    .image_status(tonic::Request::new(req))
                    .await?
                    .into_inner();

                resp.image = resp.image.map(|img| {
                    let spec = img.spec.map(|s| ImageSpec {
                        runtime_handler: "xpu".to_string(),
                        ..s
                    });

                    Image { spec, ..img }
                });

                debug!("image_status response in xpu: {:?}", resp);

                return Ok(tonic::Response::new(resp));
            }
        }

        let mut client = self.host_client.clone();

        client.image_status(tonic::Request::new(req)).await
    }

    /// PullImage pulls an image with authentication config.
    async fn pull_image(
        &self,
        request: tonic::Request<PullImageRequest>,
    ) -> Result<tonic::Response<PullImageResponse>, tonic::Status> {
        trace_fn!("ImageShim::pull_image");
        let req = request.into_inner();
        debug!("pull_image request: {:?}", req);

        if let Some(img) = req.image.clone() {
            if img.runtime_handler.as_str() == "xpu" {
                let mut client = self.xpu_client.clone();

                return client.pull_image(tonic::Request::new(req)).await;
            }
        }

        let mut client = self.host_client.clone();

        client.pull_image(tonic::Request::new(req)).await
    }

    async fn remove_image(
        &self,
        request: tonic::Request<RemoveImageRequest>,
    ) -> Result<tonic::Response<RemoveImageResponse>, tonic::Status> {
        trace_fn!("ImageShim::remove_image");
        let req = request.into_inner();
        debug!("remove_image request: {:?}", req);

        if let Some(img) = req.image.clone() {
            if img.runtime_handler.as_str() == "xpu" {
                let mut client = self.xpu_client.clone();

                return client.remove_image(tonic::Request::new(req)).await;
            }
        }

        let mut client = self.host_client.clone();

        client.remove_image(tonic::Request::new(req)).await
    }

    /// ImageFSInfo returns information of the filesystem that is used to store images.
    async fn image_fs_info(
        &self,
        request: tonic::Request<ImageFsInfoRequest>,
    ) -> Result<tonic::Response<ImageFsInfoResponse>, tonic::Status> {
        trace_fn!("ImageShim::image_fs_info");
        let mut client = self.host_client.clone();

        client.image_fs_info(request).await
    }
}
