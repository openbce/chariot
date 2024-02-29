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
use std::collections::HashSet;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::Stream;
use stdng::{logs::TraceFn, trace_fn};
use tokio::net::UnixStream;
use tonic::transport::Channel;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use tracing::{debug, info};

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
    pub xpu_containers: Arc<Mutex<HashSet<String>>>,
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
            xpu_containers: Arc::new(Mutex::new(HashSet::new())),
        })
    }
}

impl RuntimeShim {
    fn set_container(&self, id: String) {
        let mut cp = self.xpu_containers.lock().unwrap();
        cp.insert(id);
    }

    fn del_container(&self, id: String) {
        let mut cp = self.xpu_containers.lock().unwrap();
        cp.remove(&id);
    }

    fn is_container(&self, id: String) -> bool {
        let cp = self.xpu_containers.lock().unwrap();
        cp.contains(&id)
    }
}

#[async_trait::async_trait]
impl crirpc::runtime_service_server::RuntimeService for RuntimeShim {
    async fn version(
        &self,
        request: tonic::Request<VersionRequest>,
    ) -> Result<tonic::Response<VersionResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::version");
        let mut client = self.host_client.clone();

        client.version(request).await
    }

    async fn run_pod_sandbox(
        &self,
        request: tonic::Request<RunPodSandboxRequest>,
    ) -> Result<tonic::Response<RunPodSandboxResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::run_pod_sandbox");

        let mut req = request.into_inner();
        let resp = match req.runtime_handler.as_str() {
            "xpu" => {
                debug!("run_pod_sandbox request in xpu: {:?}", req);
                let mut client = self.xpu_client.clone();
                req.runtime_handler = String::new();

                let resp = client.run_pod_sandbox(tonic::Request::new(req)).await?;
                let resp = resp.into_inner();
                self.set_container(resp.pod_sandbox_id.clone());

                resp
            }
            _ => {
                debug!("run_pod_sandbox request in host: {:?}", req);
                let mut client = self.host_client.clone();

                client
                    .run_pod_sandbox(tonic::Request::new(req))
                    .await?
                    .into_inner()
            }
        };

        debug!("run_pod_sandbox response: {:?}", resp);

        Ok(tonic::Response::new(resp))
    }

    async fn stop_pod_sandbox(
        &self,
        request: tonic::Request<StopPodSandboxRequest>,
    ) -> Result<tonic::Response<StopPodSandboxResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::stop_pod_sandbox");

        let req = request.into_inner();
        let mut client = match self.is_container(req.pod_sandbox_id.clone()) {
            true => self.xpu_client.clone(),
            false => self.host_client.clone(),
        };

        client.stop_pod_sandbox(tonic::Request::new(req)).await
    }

    async fn remove_pod_sandbox(
        &self,
        request: tonic::Request<RemovePodSandboxRequest>,
    ) -> Result<tonic::Response<RemovePodSandboxResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::remove_pod_sandbox");

        let req = request.into_inner();
        let id = req.pod_sandbox_id.clone();

        let mut client = match self.is_container(req.pod_sandbox_id.clone()) {
            true => self.xpu_client.clone(),
            false => self.host_client.clone(),
        };

        let resp = client.remove_pod_sandbox(tonic::Request::new(req)).await?;
        let resp = resp.into_inner();

        self.del_container(id);

        Ok(tonic::Response::new(resp))
    }

    async fn pod_sandbox_status(
        &self,
        request: tonic::Request<PodSandboxStatusRequest>,
    ) -> Result<tonic::Response<PodSandboxStatusResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::pod_sandbox_status");

        let req = request.into_inner();
        debug!("pod_sandbox_status request: {:?}", req);
        let mut client = match self.is_container(req.pod_sandbox_id.clone()) {
            true => self.xpu_client.clone(),
            false => self.host_client.clone(),
        };

        let resp = client
            .pod_sandbox_status(tonic::Request::new(req))
            .await?
            .into_inner();

        debug!("pod_sandbox_status response: {:?}", resp);

        Ok(tonic::Response::new(resp))
    }

    /// ListPodSandbox returns a list of PodSandboxes.
    async fn list_pod_sandbox(
        &self,
        request: tonic::Request<ListPodSandboxRequest>,
    ) -> Result<tonic::Response<ListPodSandboxResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::list_pod_sandbox");

        let req = request.into_inner();
        let mut host_client = self.host_client.clone();
        let mut xpu_client = self.xpu_client.clone();

        let host_pods = host_client
            .list_pod_sandbox(req.clone())
            .await?
            .into_inner();
        let xpu_pods = xpu_client.list_pod_sandbox(req.clone()).await?.into_inner();

        let mut resp = ListPodSandboxResponse { items: vec![] };

        resp.items.extend(host_pods.items);
        resp.items.extend(xpu_pods.items);

        Ok(tonic::Response::new(resp))
    }

    /// CreateContainer creates a new container in specified PodSandbox
    async fn create_container(
        &self,
        request: tonic::Request<CreateContainerRequest>,
    ) -> Result<tonic::Response<CreateContainerResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::create_container");

        let req = request.into_inner();
        debug!("create_container request: {:?}", req);

        let resp = match self.is_container(req.pod_sandbox_id.clone()) {
            true => {
                let mut client = self.xpu_client.clone();
                let resp = client.create_container(tonic::Request::new(req)).await?;
                let resp = resp.into_inner();

                self.set_container(resp.container_id.clone());

                resp
            }
            false => {
                let mut client = self.host_client.clone();

                client
                    .create_container(tonic::Request::new(req))
                    .await?
                    .into_inner()
            }
        };

        debug!("create_container response: {:?}", resp);

        Ok(tonic::Response::new(resp))
    }

    /// StartContainer starts the container.
    async fn start_container(
        &self,
        request: tonic::Request<StartContainerRequest>,
    ) -> Result<tonic::Response<StartContainerResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::start_container");

        let req = request.into_inner();
        debug!("start_container requet is: {:?}", req);
        let mut client = match self.is_container(req.container_id.clone()) {
            true => self.xpu_client.clone(),
            false => self.host_client.clone(),
        };

        let resp = client
            .start_container(tonic::Request::new(req))
            .await?;

        debug!("start_container response is: {:?}", resp.get_ref());

        Ok(resp)
    }

    async fn stop_container(
        &self,
        request: tonic::Request<StopContainerRequest>,
    ) -> Result<tonic::Response<StopContainerResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::stop_container");

        let req = request.into_inner();
        debug!("stop_container requet is: {:?}", req);
        let mut client = match self.is_container(req.container_id.clone()) {
            true => self.xpu_client.clone(),
            false => self.host_client.clone(),
        };

        let resp = client.stop_container(tonic::Request::new(req)).await?;

        debug!("stop_container response is: {:?}", resp.get_ref());

        Ok(resp)
    }

    async fn remove_container(
        &self,
        request: tonic::Request<RemoveContainerRequest>,
    ) -> Result<tonic::Response<RemoveContainerResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::remove_container");

        let req = request.into_inner();
        match self.is_container(req.container_id.clone()) {
            true => {
                let mut client = self.xpu_client.clone();
                let resp = client
                    .remove_container(tonic::Request::new(req.clone()))
                    .await?;
                let resp = resp.into_inner();

                self.del_container(req.container_id.clone());

                Ok(tonic::Response::new(resp))
            }
            false => {
                let mut client = self.host_client.clone();

                client.remove_container(tonic::Request::new(req)).await
            }
        }
    }
    /// ListContainers lists all containers by filters.
    async fn list_containers(
        &self,
        request: tonic::Request<ListContainersRequest>,
    ) -> Result<tonic::Response<ListContainersResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::list_containers");

        let mut host_client = self.host_client.clone();
        let mut xpu_client = self.xpu_client.clone();

        let req = request.into_inner();

        let host_containers = host_client
            .list_containers(tonic::Request::new(req.clone()))
            .await?
            .into_inner();
        let xpu_containers = xpu_client
            .list_containers(tonic::Request::new(req))
            .await?
            .into_inner();

        let mut containers = vec![];

        containers.extend(xpu_containers.containers);
        containers.extend(host_containers.containers);

        Ok(tonic::Response::new(ListContainersResponse { containers }))
    }

    async fn container_status(
        &self,
        request: tonic::Request<ContainerStatusRequest>,
    ) -> Result<tonic::Response<ContainerStatusResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::container_status");

        let req = request.into_inner();
        let mut client = match self.is_container(req.container_id.clone()) {
            false => self.host_client.clone(),
            true => self.xpu_client.clone(),
        };

        client.container_status(tonic::Request::new(req)).await
    }

    async fn update_container_resources(
        &self,
        request: tonic::Request<UpdateContainerResourcesRequest>,
    ) -> Result<tonic::Response<UpdateContainerResourcesResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::update_container_resources");

        let req = request.into_inner();
        let mut client = match self.is_container(req.container_id.clone()) {
            false => self.host_client.clone(),
            true => self.xpu_client.clone(),
        };

        client
            .update_container_resources(tonic::Request::new(req))
            .await
    }

    async fn reopen_container_log(
        &self,
        request: tonic::Request<ReopenContainerLogRequest>,
    ) -> Result<tonic::Response<ReopenContainerLogResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::reopen_container_log");

        let req = request.into_inner();
        let mut client = match self.is_container(req.container_id.clone()) {
            false => self.host_client.clone(),
            true => self.xpu_client.clone(),
        };

        client.reopen_container_log(tonic::Request::new(req)).await
    }
    /// ExecSync runs a command in a container synchronously.
    async fn exec_sync(
        &self,
        request: tonic::Request<ExecSyncRequest>,
    ) -> Result<tonic::Response<ExecSyncResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::exec_sync");

        let req = request.into_inner();
        let mut client = match self.is_container(req.container_id.clone()) {
            false => self.host_client.clone(),
            true => self.xpu_client.clone(),
        };

        client.exec_sync(tonic::Request::new(req)).await
    }
    /// Exec prepares a streaming endpoint to execute a command in the container.
    async fn exec(
        &self,
        request: tonic::Request<ExecRequest>,
    ) -> Result<tonic::Response<ExecResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::exec");

        let req = request.into_inner();
        let mut client = match self.is_container(req.container_id.clone()) {
            false => self.host_client.clone(),
            true => self.xpu_client.clone(),
        };

        client.exec(tonic::Request::new(req)).await
    }
    /// Attach prepares a streaming endpoint to attach to a running container.
    async fn attach(
        &self,
        request: tonic::Request<AttachRequest>,
    ) -> Result<tonic::Response<AttachResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::attach");

        let req = request.into_inner();
        let mut client = match self.is_container(req.container_id.clone()) {
            false => self.host_client.clone(),
            true => self.xpu_client.clone(),
        };

        client.attach(tonic::Request::new(req)).await
    }
    /// PortForward prepares a streaming endpoint to forward ports from a PodSandbox.
    async fn port_forward(
        &self,
        request: tonic::Request<PortForwardRequest>,
    ) -> Result<tonic::Response<PortForwardResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::port_forward");

        let req = request.into_inner();
        let mut client = match self.is_container(req.pod_sandbox_id.clone()) {
            false => self.host_client.clone(),
            true => self.xpu_client.clone(),
        };

        client.port_forward(tonic::Request::new(req)).await
    }

    async fn container_stats(
        &self,
        request: tonic::Request<ContainerStatsRequest>,
    ) -> Result<tonic::Response<ContainerStatsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::container_stats");

        let req = request.into_inner();
        let mut client = match self.is_container(req.container_id.clone()) {
            false => self.host_client.clone(),
            true => self.xpu_client.clone(),
        };

        client.container_stats(tonic::Request::new(req)).await
    }

    /// ListContainerStats returns stats of all running containers.
    async fn list_container_stats(
        &self,
        request: tonic::Request<ListContainerStatsRequest>,
    ) -> Result<tonic::Response<ListContainerStatsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::list_container_stats");

        let req = request.into_inner();
        let mut host_client = self.host_client.clone();
        let mut xpu_client = self.xpu_client.clone();

        let host_containers = host_client
            .list_container_stats(tonic::Request::new(req.clone()))
            .await?
            .into_inner();
        let xpu_containers = xpu_client
            .list_container_stats(tonic::Request::new(req))
            .await?
            .into_inner();

        let mut stats = vec![];

        stats.extend(host_containers.stats);
        stats.extend(xpu_containers.stats);

        Ok(tonic::Response::new(ListContainerStatsResponse { stats }))
    }

    async fn pod_sandbox_stats(
        &self,
        request: tonic::Request<PodSandboxStatsRequest>,
    ) -> Result<tonic::Response<PodSandboxStatsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::pod_sandbox_stats");

        let req = request.into_inner();
        let mut client = match self.is_container(req.pod_sandbox_id.clone()) {
            false => self.host_client.clone(),
            true => self.xpu_client.clone(),
        };

        client.pod_sandbox_stats(tonic::Request::new(req)).await
    }
    /// ListPodSandboxStats returns stats of the pod sandboxes matching a filter.
    async fn list_pod_sandbox_stats(
        &self,
        request: tonic::Request<ListPodSandboxStatsRequest>,
    ) -> Result<tonic::Response<ListPodSandboxStatsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::list_pod_sandbox_stats");

        let req = request.into_inner();
        let mut host_client = self.host_client.clone();
        let mut xpu_client = self.xpu_client.clone();

        let host_containers = host_client
            .list_pod_sandbox_stats(tonic::Request::new(req.clone()))
            .await?
            .into_inner();
        let xpu_containers = xpu_client
            .list_pod_sandbox_stats(tonic::Request::new(req))
            .await?
            .into_inner();

        let mut stats = vec![];

        stats.extend(host_containers.stats);
        stats.extend(xpu_containers.stats);

        Ok(tonic::Response::new(ListPodSandboxStatsResponse { stats }))
    }

    /// UpdateRuntimeConfig updates the runtime configuration based on the given request.
    async fn update_runtime_config(
        &self,
        request: tonic::Request<UpdateRuntimeConfigRequest>,
    ) -> Result<tonic::Response<UpdateRuntimeConfigResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::update_runtime_config");

        let req = request.into_inner();
        let mut client = self.host_client.clone();
        client.update_runtime_config(tonic::Request::new(req.clone())).await?;

        let mut client = self.xpu_client.clone();
        client.update_runtime_config(tonic::Request::new(req)).await?;

        Ok(tonic::Response::new(UpdateRuntimeConfigResponse{}))
    }
    /// Status returns the status of the runtime.
    async fn status(
        &self,
        request: tonic::Request<StatusRequest>,
    ) -> Result<tonic::Response<StatusResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::status");

        let mut client = self.host_client.clone();

        client.status(request).await
    }
    /// CheckpointContainer checkpoints a container
    async fn checkpoint_container(
        &self,
        request: tonic::Request<CheckpointContainerRequest>,
    ) -> Result<tonic::Response<CheckpointContainerResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::checkpoint_container");

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
        trace_fn!("RuntimeShim::get_container_events");
        todo!()
    }

    async fn list_metric_descriptors(
        &self,
        request: tonic::Request<ListMetricDescriptorsRequest>,
    ) -> Result<tonic::Response<ListMetricDescriptorsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::list_metric_descriptors");

        let mut client = self.host_client.clone();

        client.list_metric_descriptors(request).await
    }
    /// ListPodSandboxMetrics gets pod sandbox metrics from CRI Runtime
    async fn list_pod_sandbox_metrics(
        &self,
        request: tonic::Request<ListPodSandboxMetricsRequest>,
    ) -> Result<tonic::Response<ListPodSandboxMetricsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::list_pod_sandbox_metrics");

        let mut client = self.host_client.clone();

        client.list_pod_sandbox_metrics(request).await
    }

    async fn runtime_config(
        &self,
        request: tonic::Request<RuntimeConfigRequest>,
    ) -> Result<tonic::Response<RuntimeConfigResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::runtime_config");

        let mut client = self.host_client.clone();

        client.runtime_config(request).await
    }
}
