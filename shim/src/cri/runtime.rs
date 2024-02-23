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
use std::pin::Pin;

use futures::Stream;
use tokio::net::UnixStream;
use tonic::transport::Channel;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use tracing::info;

use self::crirpc::runtime_service_client::RuntimeServiceClient;
use self::crirpc::{
    AttachRequest, AttachResponse, CheckpointContainerRequest, CheckpointContainerResponse,
    ContainerEventResponse, ContainerStatsRequest, ContainerStatsResponse, ContainerStatusRequest,
    ContainerStatusResponse, CreateContainerRequest, CreateContainerResponse, ExecRequest,
    ExecResponse, ExecSyncRequest, ExecSyncResponse, GetEventsRequest, ListContainerStatsRequest,
    ListContainerStatsResponse, ListContainersRequest, ListContainersResponse,
    ListMetricDescriptorsRequest, ListMetricDescriptorsResponse, ListPodSandboxMetricsRequest,
    ListPodSandboxMetricsResponse, ListPodSandboxRequest, ListPodSandboxResponse,
    ListPodSandboxStatsRequest, ListPodSandboxStatsResponse, PodSandboxStatsRequest,
    PodSandboxStatsResponse, PodSandboxStatusRequest, PodSandboxStatusResponse, PortForwardRequest,
    PortForwardResponse, RemoveContainerRequest, RemoveContainerResponse, RemovePodSandboxRequest,
    RemovePodSandboxResponse, ReopenContainerLogRequest, ReopenContainerLogResponse,
    RunPodSandboxRequest, RunPodSandboxResponse, RuntimeConfigRequest, RuntimeConfigResponse,
    StartContainerRequest, StartContainerResponse, StatusRequest, StatusResponse,
    StopContainerRequest, StopContainerResponse, StopPodSandboxRequest, StopPodSandboxResponse,
    UpdateContainerResourcesRequest, UpdateContainerResourcesResponse, UpdateRuntimeConfigRequest,
    UpdateRuntimeConfigResponse, VersionRequest, VersionResponse,
};
use crate::rpc::cri as crirpc;

use crate::common::ChariotError;

pub struct RuntimeShim {
    pub xpu_client: RuntimeServiceClient<Channel>,
    pub host_client: RuntimeServiceClient<Channel>,
}

impl RuntimeShim {
    pub async fn connect(host_cri: String, xpu_cri: String) -> Result<Self, ChariotError> {
        let mut xpu_client = RuntimeServiceClient::connect(xpu_cri)
            .await
            .map_err(|e| ChariotError::NetworkError(e.to_string()))?;

        let channel = Endpoint::try_from("http://[::]:50051")
            .map_err(|e| ChariotError::NetworkError(e.to_string()))?
            .connect_with_connector(service_fn(move |_: Uri| {
                let host_path = host_cri.clone();
                UnixStream::connect(host_path)
            }))
            .await
            .map_err(|e| ChariotError::NetworkError(e.to_string()))?;
        let mut host_client = RuntimeServiceClient::new(channel);

        let request = crirpc::VersionRequest {
            version: "*".to_string(),
        };

        let version = xpu_client
            .version(request.clone())
            .await
            .map_err(|e| ChariotError::CriError(e.to_string()))?;
        let resp = version.into_inner();

        info!(
            "XPU runtime: {}/{}",
            resp.runtime_name, resp.runtime_version
        );

        let version = host_client
            .version(request.clone())
            .await
            .map_err(|e| ChariotError::CriError(e.to_string()))?;
        let resp = version.into_inner();

        info!(
            "Host runtime: {}/{}",
            resp.runtime_name, resp.runtime_version
        );

        Ok(RuntimeShim {
            xpu_client,
            host_client,
        })
    }
}

#[async_trait::async_trait]
impl crirpc::runtime_service_server::RuntimeService for RuntimeShim {
    async fn version(
        &self,
        request: tonic::Request<VersionRequest>,
    ) -> Result<tonic::Response<VersionResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.version(request).await
    }

    async fn run_pod_sandbox(
        &self,
        request: tonic::Request<RunPodSandboxRequest>,
    ) -> Result<tonic::Response<RunPodSandboxResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.run_pod_sandbox(request).await
    }

    async fn stop_pod_sandbox(
        &self,
        request: tonic::Request<StopPodSandboxRequest>,
    ) -> Result<tonic::Response<StopPodSandboxResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.stop_pod_sandbox(request).await
    }

    async fn remove_pod_sandbox(
        &self,
        request: tonic::Request<RemovePodSandboxRequest>,
    ) -> Result<tonic::Response<RemovePodSandboxResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.remove_pod_sandbox(request).await
    }

    async fn pod_sandbox_status(
        &self,
        request: tonic::Request<PodSandboxStatusRequest>,
    ) -> Result<tonic::Response<PodSandboxStatusResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.pod_sandbox_status(request).await
    }
    /// ListPodSandbox returns a list of PodSandboxes.
    async fn list_pod_sandbox(
        &self,
        request: tonic::Request<ListPodSandboxRequest>,
    ) -> Result<tonic::Response<ListPodSandboxResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.list_pod_sandbox(request).await
    }
    /// CreateContainer creates a new container in specified PodSandbox
    async fn create_container(
        &self,
        request: tonic::Request<CreateContainerRequest>,
    ) -> Result<tonic::Response<CreateContainerResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.create_container(request).await
    }
    /// StartContainer starts the container.
    async fn start_container(
        &self,
        request: tonic::Request<StartContainerRequest>,
    ) -> Result<tonic::Response<StartContainerResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.start_container(request).await
    }

    async fn stop_container(
        &self,
        request: tonic::Request<StopContainerRequest>,
    ) -> Result<tonic::Response<StopContainerResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.stop_container(request).await
    }

    async fn remove_container(
        &self,
        request: tonic::Request<RemoveContainerRequest>,
    ) -> Result<tonic::Response<RemoveContainerResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.remove_container(request).await
    }
    /// ListContainers lists all containers by filters.
    async fn list_containers(
        &self,
        request: tonic::Request<ListContainersRequest>,
    ) -> Result<tonic::Response<ListContainersResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.list_containers(request).await
    }

    async fn container_status(
        &self,
        request: tonic::Request<ContainerStatusRequest>,
    ) -> Result<tonic::Response<ContainerStatusResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.container_status(request).await
    }

    async fn update_container_resources(
        &self,
        request: tonic::Request<UpdateContainerResourcesRequest>,
    ) -> Result<tonic::Response<UpdateContainerResourcesResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.update_container_resources(request).await
    }

    async fn reopen_container_log(
        &self,
        request: tonic::Request<ReopenContainerLogRequest>,
    ) -> Result<tonic::Response<ReopenContainerLogResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.reopen_container_log(request).await
    }
    /// ExecSync runs a command in a container synchronously.
    async fn exec_sync(
        &self,
        request: tonic::Request<ExecSyncRequest>,
    ) -> Result<tonic::Response<ExecSyncResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.exec_sync(request).await
    }
    /// Exec prepares a streaming endpoint to execute a command in the container.
    async fn exec(
        &self,
        request: tonic::Request<ExecRequest>,
    ) -> Result<tonic::Response<ExecResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.exec(request).await
    }
    /// Attach prepares a streaming endpoint to attach to a running container.
    async fn attach(
        &self,
        request: tonic::Request<AttachRequest>,
    ) -> Result<tonic::Response<AttachResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.attach(request).await
    }
    /// PortForward prepares a streaming endpoint to forward ports from a PodSandbox.
    async fn port_forward(
        &self,
        request: tonic::Request<PortForwardRequest>,
    ) -> Result<tonic::Response<PortForwardResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.port_forward(request).await
    }

    async fn container_stats(
        &self,
        request: tonic::Request<ContainerStatsRequest>,
    ) -> Result<tonic::Response<ContainerStatsResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.container_stats(request).await
    }
    /// ListContainerStats returns stats of all running containers.
    async fn list_container_stats(
        &self,
        request: tonic::Request<ListContainerStatsRequest>,
    ) -> Result<tonic::Response<ListContainerStatsResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.list_container_stats(request).await
    }

    async fn pod_sandbox_stats(
        &self,
        request: tonic::Request<PodSandboxStatsRequest>,
    ) -> Result<tonic::Response<PodSandboxStatsResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.pod_sandbox_stats(request).await
    }
    /// ListPodSandboxStats returns stats of the pod sandboxes matching a filter.
    async fn list_pod_sandbox_stats(
        &self,
        request: tonic::Request<ListPodSandboxStatsRequest>,
    ) -> Result<tonic::Response<ListPodSandboxStatsResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.list_pod_sandbox_stats(request).await
    }
    /// UpdateRuntimeConfig updates the runtime configuration based on the given request.
    async fn update_runtime_config(
        &self,
        request: tonic::Request<UpdateRuntimeConfigRequest>,
    ) -> Result<tonic::Response<UpdateRuntimeConfigResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.update_runtime_config(request).await
    }
    /// Status returns the status of the runtime.
    async fn status(
        &self,
        request: tonic::Request<StatusRequest>,
    ) -> Result<tonic::Response<StatusResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.status(request).await
    }
    /// CheckpointContainer checkpoints a container
    async fn checkpoint_container(
        &self,
        request: tonic::Request<CheckpointContainerRequest>,
    ) -> Result<tonic::Response<CheckpointContainerResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.checkpoint_container(request).await
    }

    type GetContainerEventsStream =
        Pin<Box<dyn Stream<Item = Result<ContainerEventResponse, tonic::Status>> + Send + 'static>>;

    /// GetContainerEvents gets container events from the CRI runtime
    async fn get_container_events(
        &self,
        _request: tonic::Request<GetEventsRequest>,
    ) -> Result<tonic::Response<Self::GetContainerEventsStream>, tonic::Status> {
        todo!()
    }

    async fn list_metric_descriptors(
        &self,
        request: tonic::Request<ListMetricDescriptorsRequest>,
    ) -> Result<tonic::Response<ListMetricDescriptorsResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.list_metric_descriptors(request).await
    }
    /// ListPodSandboxMetrics gets pod sandbox metrics from CRI Runtime
    async fn list_pod_sandbox_metrics(
        &self,
        request: tonic::Request<ListPodSandboxMetricsRequest>,
    ) -> Result<tonic::Response<ListPodSandboxMetricsResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.list_pod_sandbox_metrics(request).await
    }

    async fn runtime_config(
        &self,
        request: tonic::Request<RuntimeConfigRequest>,
    ) -> Result<tonic::Response<RuntimeConfigResponse>, tonic::Status> {
        let mut client = self.host_client.clone();

        client.runtime_config(request).await
    }
}
