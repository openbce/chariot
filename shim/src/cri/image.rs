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

use tonic::transport::Channel;
use tracing::info;

use self::crirpc::image_service_client::ImageServiceClient;
use self::crirpc::{
    ImageFsInfoRequest, ImageFsInfoResponse, ImageStatusRequest, ImageStatusResponse,
    ListImagesRequest, ListImagesResponse, PullImageRequest, PullImageResponse, RemoveImageRequest,
    RemoveImageResponse,
};
use crate::rpc::cri as crirpc;

use crate::common::ChariotError;

pub struct ImageShim {
    pub xpu_client: ImageServiceClient<Channel>,
}

impl ImageShim {
    pub async fn connect(cri_addr: String) -> Result<Self, ChariotError> {
        let mut image_client = ImageServiceClient::connect(cri_addr)
            .await
            .map_err(|e| ChariotError::NetworkError(e.to_string()))?;

        let resp = image_client
            .image_fs_info(ImageFsInfoRequest {})
            .await
            .map_err(|e| ChariotError::CriError(e.to_string()))?;
        let fs_info = resp.into_inner();

        for fs in fs_info.container_filesystems {
            info!("Container FS: {:?}", fs.fs_id.map(|i| i.mountpoint),);
        }

        for fs in fs_info.image_filesystems {
            info!("Image FS: {:?}", fs.fs_id.map(|i| i.mountpoint),);
        }

        Ok(ImageShim {
            xpu_client: image_client,
        })
    }
}

#[async_trait::async_trait]
impl crirpc::image_service_server::ImageService for ImageShim {
    async fn list_images(
        &self,
        request: tonic::Request<ListImagesRequest>,
    ) -> Result<tonic::Response<ListImagesResponse>, tonic::Status> {
        let mut client = self.xpu_client.clone();

        client.list_images(request).await
    }

    async fn image_status(
        &self,
        request: tonic::Request<ImageStatusRequest>,
    ) -> Result<tonic::Response<ImageStatusResponse>, tonic::Status> {
        let mut client = self.xpu_client.clone();

        client.image_status(request).await
    }
    /// PullImage pulls an image with authentication config.
    async fn pull_image(
        &self,
        request: tonic::Request<PullImageRequest>,
    ) -> Result<tonic::Response<PullImageResponse>, tonic::Status> {
        let mut client = self.xpu_client.clone();

        client.pull_image(request).await
    }

    async fn remove_image(
        &self,
        request: tonic::Request<RemoveImageRequest>,
    ) -> Result<tonic::Response<RemoveImageResponse>, tonic::Status> {
        let mut client = self.xpu_client.clone();

        client.remove_image(request).await
    }
    /// ImageFSInfo returns information of the filesystem that is used to store images.
    async fn image_fs_info(
        &self,
        request: tonic::Request<ImageFsInfoRequest>,
    ) -> Result<tonic::Response<ImageFsInfoResponse>, tonic::Status> {
        let mut client = self.xpu_client.clone();

        client.image_fs_info(request).await
    }
}
