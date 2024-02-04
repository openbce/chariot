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

use crate::crirpc::{
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

pub struct ChariotShim {}

#[async_trait::async_trait]
impl crirpc::runtime_service_server::RuntimeService for ChariotShim {
    async fn version(
        &self,
        request: tonic::Request<VersionRequest>,
    ) -> Result<tonic::Response<VersionResponse>, tonic::Status> {
        todo!();
    }

    async fn run_pod_sandbox(
        &self,
        request: tonic::Request<RunPodSandboxRequest>,
    ) -> Result<tonic::Response<RunPodSandboxResponse>, tonic::Status> {
        todo!()
    }

    async fn stop_pod_sandbox(
        &self,
        request: tonic::Request<StopPodSandboxRequest>,
    ) -> Result<tonic::Response<StopPodSandboxResponse>, tonic::Status> {
        todo!()
    }

    async fn remove_pod_sandbox(
        &self,
        request: tonic::Request<RemovePodSandboxRequest>,
    ) -> Result<tonic::Response<RemovePodSandboxResponse>, tonic::Status> {
        todo!()
    }

    async fn pod_sandbox_status(
        &self,
        request: tonic::Request<PodSandboxStatusRequest>,
    ) -> Result<tonic::Response<PodSandboxStatusResponse>, tonic::Status> {
        todo!()
    }
    /// ListPodSandbox returns a list of PodSandboxes.
    async fn list_pod_sandbox(
        &self,
        request: tonic::Request<ListPodSandboxRequest>,
    ) -> Result<tonic::Response<ListPodSandboxResponse>, tonic::Status> {
        todo!()
    }
    /// CreateContainer creates a new container in specified PodSandbox
    async fn create_container(
        &self,
        request: tonic::Request<CreateContainerRequest>,
    ) -> Result<tonic::Response<CreateContainerResponse>, tonic::Status> {
        todo!()
    }
    /// StartContainer starts the container.
    async fn start_container(
        &self,
        request: tonic::Request<StartContainerRequest>,
    ) -> Result<tonic::Response<StartContainerResponse>, tonic::Status> {
        todo!()
    }

    async fn stop_container(
        &self,
        request: tonic::Request<StopContainerRequest>,
    ) -> Result<tonic::Response<StopContainerResponse>, tonic::Status> {
        todo!()
    }

    async fn remove_container(
        &self,
        request: tonic::Request<RemoveContainerRequest>,
    ) -> Result<tonic::Response<RemoveContainerResponse>, tonic::Status> {
        todo!()
    }
    /// ListContainers lists all containers by filters.
    async fn list_containers(
        &self,
        request: tonic::Request<ListContainersRequest>,
    ) -> Result<tonic::Response<ListContainersResponse>, tonic::Status> {
        todo!()
    }

    async fn container_status(
        &self,
        request: tonic::Request<ContainerStatusRequest>,
    ) -> Result<tonic::Response<ContainerStatusResponse>, tonic::Status> {
        todo!()
    }

    async fn update_container_resources(
        &self,
        request: tonic::Request<UpdateContainerResourcesRequest>,
    ) -> Result<tonic::Response<UpdateContainerResourcesResponse>, tonic::Status> {
        todo!()
    }

    async fn reopen_container_log(
        &self,
        request: tonic::Request<ReopenContainerLogRequest>,
    ) -> Result<tonic::Response<ReopenContainerLogResponse>, tonic::Status> {
        todo!()
    }
    /// ExecSync runs a command in a container synchronously.
    async fn exec_sync(
        &self,
        request: tonic::Request<ExecSyncRequest>,
    ) -> Result<tonic::Response<ExecSyncResponse>, tonic::Status> {
        todo!()
    }
    /// Exec prepares a streaming endpoint to execute a command in the container.
    async fn exec(
        &self,
        request: tonic::Request<ExecRequest>,
    ) -> Result<tonic::Response<ExecResponse>, tonic::Status> {
        todo!()
    }
    /// Attach prepares a streaming endpoint to attach to a running container.
    async fn attach(
        &self,
        request: tonic::Request<AttachRequest>,
    ) -> Result<tonic::Response<AttachResponse>, tonic::Status> {
        todo!()
    }
    /// PortForward prepares a streaming endpoint to forward ports from a PodSandbox.
    async fn port_forward(
        &self,
        request: tonic::Request<PortForwardRequest>,
    ) -> Result<tonic::Response<PortForwardResponse>, tonic::Status> {
        todo!()
    }

    async fn container_stats(
        &self,
        request: tonic::Request<ContainerStatsRequest>,
    ) -> Result<tonic::Response<ContainerStatsResponse>, tonic::Status> {
        todo!()
    }
    /// ListContainerStats returns stats of all running containers.
    async fn list_container_stats(
        &self,
        request: tonic::Request<ListContainerStatsRequest>,
    ) -> Result<tonic::Response<ListContainerStatsResponse>, tonic::Status> {
        todo!()
    }

    async fn pod_sandbox_stats(
        &self,
        request: tonic::Request<PodSandboxStatsRequest>,
    ) -> Result<tonic::Response<PodSandboxStatsResponse>, tonic::Status> {
        todo!()
    }
    /// ListPodSandboxStats returns stats of the pod sandboxes matching a filter.
    async fn list_pod_sandbox_stats(
        &self,
        request: tonic::Request<ListPodSandboxStatsRequest>,
    ) -> Result<tonic::Response<ListPodSandboxStatsResponse>, tonic::Status> {
        todo!()
    }
    /// UpdateRuntimeConfig updates the runtime configuration based on the given request.
    async fn update_runtime_config(
        &self,
        request: tonic::Request<UpdateRuntimeConfigRequest>,
    ) -> Result<tonic::Response<UpdateRuntimeConfigResponse>, tonic::Status> {
        todo!()
    }
    /// Status returns the status of the runtime.
    async fn status(
        &self,
        request: tonic::Request<StatusRequest>,
    ) -> Result<tonic::Response<StatusResponse>, tonic::Status> {
        todo!()
    }
    /// CheckpointContainer checkpoints a container
    async fn checkpoint_container(
        &self,
        request: tonic::Request<CheckpointContainerRequest>,
    ) -> Result<tonic::Response<CheckpointContainerResponse>, tonic::Status> {
        todo!()
    }

    type GetContainerEventsStream =
        Pin<Box<dyn Stream<Item = Result<ContainerEventResponse, tonic::Status>> + Send + 'static>>;

    /// GetContainerEvents gets container events from the CRI runtime
    async fn get_container_events(
        &self,
        request: tonic::Request<GetEventsRequest>,
    ) -> Result<tonic::Response<Self::GetContainerEventsStream>, tonic::Status> {
        todo!()
    }

    async fn list_metric_descriptors(
        &self,
        request: tonic::Request<ListMetricDescriptorsRequest>,
    ) -> Result<tonic::Response<ListMetricDescriptorsResponse>, tonic::Status> {
        todo!()
    }
    /// ListPodSandboxMetrics gets pod sandbox metrics from CRI Runtime
    async fn list_pod_sandbox_metrics(
        &self,
        request: tonic::Request<ListPodSandboxMetricsRequest>,
    ) -> Result<tonic::Response<ListPodSandboxMetricsResponse>, tonic::Status> {
        todo!()
    }

    async fn runtime_config(
        &self,
        request: tonic::Request<RuntimeConfigRequest>,
    ) -> Result<tonic::Response<RuntimeConfigResponse>, tonic::Status> {
        todo!()
    }
}
