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
use std::pin::Pin;

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
use self::storage::Storage;
use crate::cfg::ChariotOptions;

use crate::runtime::storage::{Container, Sandbox};
use chariot::apis::ChariotError;
use chariot::cri as crirpc;

mod storage;

pub struct RuntimeShim {
    clients: HashMap<String, RuntimeServiceClient<Channel>>,
    storage: Storage,
    default_runtime: String,
}

impl RuntimeShim {
    pub async fn connect(opts: ChariotOptions) -> Result<Self, ChariotError> {
        let mut clients = HashMap::new();
        let storage = storage::new(&opts.storage)?;

        for rt in opts.runtimes {
            let uri = rt.endpoint.find("://").unwrap_or(rt.endpoint.len());
            let (protocol, path) = rt.endpoint.split_at(uri + 3);
            let host_path = path.to_string();
            let mut client = match protocol.to_lowercase().as_str() {
                "unix://" => {
                    let channel = Endpoint::try_from("http://[::]:50051")
                        .map_err(|e| ChariotError::Network(e.to_string()))?
                        .connect_with_connector(service_fn(move |_: Uri| {
                            UnixStream::connect(host_path.clone())
                        }))
                        .await
                        .map_err(|e| ChariotError::Network(e.to_string()))?;

                    RuntimeServiceClient::new(channel)
                }
                _ => RuntimeServiceClient::connect(rt.endpoint.clone())
                    .await
                    .map_err(|e| ChariotError::Network(e.to_string()))?,
            };
            // Get the version of runtime service.
            let request = crirpc::VersionRequest {
                version: "*".to_string(),
            };

            let version = client
                .version(request.clone())
                .await
                .map_err(|e| ChariotError::Cri(e.to_string()))?;
            let resp = version.into_inner();

            info!(
                "{} runtime: {}/{}",
                rt.name, resp.runtime_name, resp.runtime_version
            );

            // Sync up Sandbox and Container of each runtime.
            let request = crirpc::ListPodSandboxRequest { filter: None };
            let sandbox = client
                .list_pod_sandbox(request)
                .await
                .map_err(|e| ChariotError::Cri(e.to_string()))?;

            for s in sandbox.into_inner().items {
                storage
                    .persist_sandbox(&Sandbox {
                        id: s.id.clone(),
                        runtime: rt.name.clone(),
                    })
                    .await?;
            }

            let request = crirpc::ListContainersRequest { filter: None };
            let container = client
                .list_containers(request)
                .await
                .map_err(|e| ChariotError::Cri(e.to_string()))?;

            for c in container.into_inner().containers {
                storage
                    .persist_container(&Container {
                        id: c.id.clone(),
                        sandbox: c.pod_sandbox_id.clone(),
                        runtime: rt.name.clone(),
                    })
                    .await?;
            }

            clients.insert(rt.name, client);
        }

        let c = storage.list_containers().await?;
        let s = storage.list_sandboxes().await?;

        info!(
            "There are {} containers and {} sandboxes.",
            c.len(),
            s.len()
        );

        Ok(RuntimeShim {
            clients,
            default_runtime: opts.default,
            storage,
        })
    }
}

impl RuntimeShim {
    fn get_default_client(&self) -> Result<RuntimeServiceClient<Channel>, tonic::Status> {
        self.get_client(&self.default_runtime)
    }

    fn get_client(&self, name: &str) -> Result<RuntimeServiceClient<Channel>, tonic::Status> {
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
impl crirpc::runtime_service_server::RuntimeService for RuntimeShim {
    async fn version(
        &self,
        request: tonic::Request<VersionRequest>,
    ) -> Result<tonic::Response<VersionResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::version");
        let mut client = self.get_client(&self.default_runtime)?;

        client.version(request).await
    }

    async fn run_pod_sandbox(
        &self,
        request: tonic::Request<RunPodSandboxRequest>,
    ) -> Result<tonic::Response<RunPodSandboxResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::run_pod_sandbox");

        let req = request.into_inner();
        debug!("run_pod_sandbox request: {:?}", req);

        let mut client = self.get_client(&req.runtime_handler)?;
        let resp = client
            .run_pod_sandbox(tonic::Request::new(req.clone()))
            .await?;
        let resp = resp.into_inner();

        self.storage
            .persist_sandbox(&storage::Sandbox {
                id: resp.pod_sandbox_id.clone(),
                runtime: req.runtime_handler.clone(),
            })
            .await?;

        debug!("run_pod_sandbox response: {:?}", resp);

        Ok(tonic::Response::new(resp))
    }

    async fn stop_pod_sandbox(
        &self,
        request: tonic::Request<StopPodSandboxRequest>,
    ) -> Result<tonic::Response<StopPodSandboxResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::stop_pod_sandbox");

        let req = request.into_inner();
        let sandbox = self.storage.get_sandbox(&req.pod_sandbox_id).await?;

        let mut client = self.get_client(&sandbox.runtime)?;

        client.stop_pod_sandbox(tonic::Request::new(req)).await
    }

    async fn remove_pod_sandbox(
        &self,
        request: tonic::Request<RemovePodSandboxRequest>,
    ) -> Result<tonic::Response<RemovePodSandboxResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::remove_pod_sandbox");

        let req = request.into_inner();
        let sandbox = self.storage.get_sandbox(&req.pod_sandbox_id).await?;

        let mut client = self.get_client(&sandbox.runtime)?;
        let resp = client
            .remove_pod_sandbox(tonic::Request::new(req.clone()))
            .await?;
        self.storage.remove_sandbox(&req.pod_sandbox_id).await?;

        Ok(resp)
    }

    async fn pod_sandbox_status(
        &self,
        request: tonic::Request<PodSandboxStatusRequest>,
    ) -> Result<tonic::Response<PodSandboxStatusResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::pod_sandbox_status");

        let req = request.into_inner();
        debug!("pod_sandbox_status request: {:?}", req);

        let sandbox = self.storage.get_sandbox(&req.pod_sandbox_id).await?;
        let mut client = self.get_client(&sandbox.runtime)?;

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

        let mut resp = ListPodSandboxResponse { items: vec![] };
        for client in self.clients.values() {
            let mut client = client.clone();
            let pods = client.list_pod_sandbox(req.clone()).await?.into_inner();
            resp.items.extend(pods.items);
        }

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

        let sandbox = self.storage.get_sandbox(&req.pod_sandbox_id).await?;
        let mut client = self.get_client(&sandbox.runtime)?;
        let resp = client
            .create_container(tonic::Request::new(req.clone()))
            .await?
            .into_inner();

        self.storage
            .persist_container(&storage::Container {
                id: resp.container_id.clone(),
                sandbox: req.pod_sandbox_id.clone(),
                runtime: sandbox.runtime.clone(),
            })
            .await?;

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

        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;
        let resp = client.start_container(tonic::Request::new(req)).await?;
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

        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;
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
        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;

        let resp = client
            .remove_container(tonic::Request::new(req.clone()))
            .await?;
        self.storage.remove_container(&req.container_id).await?;

        Ok(resp)
    }

    /// ListContainers lists all containers by filters.
    async fn list_containers(
        &self,
        request: tonic::Request<ListContainersRequest>,
    ) -> Result<tonic::Response<ListContainersResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::list_containers");
        let req = request.into_inner();

        let mut containers = vec![];
        for client in self.clients.values() {
            let mut client = client.clone();
            let c = client
                .list_containers(tonic::Request::new(req.clone()))
                .await?
                .into_inner();
            containers.extend(c.containers);
        }

        Ok(tonic::Response::new(ListContainersResponse { containers }))
    }

    async fn container_status(
        &self,
        request: tonic::Request<ContainerStatusRequest>,
    ) -> Result<tonic::Response<ContainerStatusResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::container_status");

        let req = request.into_inner();
        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;

        client.container_status(tonic::Request::new(req)).await
    }

    async fn update_container_resources(
        &self,
        request: tonic::Request<UpdateContainerResourcesRequest>,
    ) -> Result<tonic::Response<UpdateContainerResourcesResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::update_container_resources");

        let req = request.into_inner();
        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;

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
        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;

        client.reopen_container_log(tonic::Request::new(req)).await
    }
    /// ExecSync runs a command in a container synchronously.
    async fn exec_sync(
        &self,
        request: tonic::Request<ExecSyncRequest>,
    ) -> Result<tonic::Response<ExecSyncResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::exec_sync");

        let req = request.into_inner();
        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;

        client.exec_sync(tonic::Request::new(req)).await
    }
    /// Exec prepares a streaming endpoint to execute a command in the container.
    async fn exec(
        &self,
        request: tonic::Request<ExecRequest>,
    ) -> Result<tonic::Response<ExecResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::exec");

        let req = request.into_inner();
        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;

        client.exec(tonic::Request::new(req)).await
    }
    /// Attach prepares a streaming endpoint to attach to a running container.
    async fn attach(
        &self,
        request: tonic::Request<AttachRequest>,
    ) -> Result<tonic::Response<AttachResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::attach");

        let req = request.into_inner();
        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;

        client.attach(tonic::Request::new(req)).await
    }
    /// PortForward prepares a streaming endpoint to forward ports from a PodSandbox.
    async fn port_forward(
        &self,
        request: tonic::Request<PortForwardRequest>,
    ) -> Result<tonic::Response<PortForwardResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::port_forward");

        let req = request.into_inner();
        let container = self.storage.get_sandbox(&req.pod_sandbox_id).await?;
        let mut client = self.get_client(&container.runtime)?;

        client.port_forward(tonic::Request::new(req)).await
    }

    async fn container_stats(
        &self,
        request: tonic::Request<ContainerStatsRequest>,
    ) -> Result<tonic::Response<ContainerStatsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::container_stats");

        let req = request.into_inner();
        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;

        client.container_stats(tonic::Request::new(req)).await
    }

    /// ListContainerStats returns stats of all running containers.
    async fn list_container_stats(
        &self,
        request: tonic::Request<ListContainerStatsRequest>,
    ) -> Result<tonic::Response<ListContainerStatsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::list_container_stats");
        let req = request.into_inner();
        let mut stats = vec![];

        for client in self.clients.values() {
            let mut client = client.clone();
            let s = client
                .list_container_stats(tonic::Request::new(req.clone()))
                .await?
                .into_inner();
            stats.extend(s.stats);
        }

        Ok(tonic::Response::new(ListContainerStatsResponse { stats }))
    }

    async fn pod_sandbox_stats(
        &self,
        request: tonic::Request<PodSandboxStatsRequest>,
    ) -> Result<tonic::Response<PodSandboxStatsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::pod_sandbox_stats");

        let req = request.into_inner();
        let sandbox = self.storage.get_sandbox(&req.pod_sandbox_id).await?;
        let mut client = self.get_client(&sandbox.runtime)?;

        client.pod_sandbox_stats(tonic::Request::new(req)).await
    }

    /// ListPodSandboxStats returns stats of the pod sandboxes matching a filter.
    async fn list_pod_sandbox_stats(
        &self,
        request: tonic::Request<ListPodSandboxStatsRequest>,
    ) -> Result<tonic::Response<ListPodSandboxStatsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::list_pod_sandbox_stats");

        let req = request.into_inner();

        let mut stats = vec![];
        for client in self.clients.values() {
            let mut client = client.clone();
            let s = client
                .list_pod_sandbox_stats(tonic::Request::new(req.clone()))
                .await?
                .into_inner();
            stats.extend(s.stats);
        }

        Ok(tonic::Response::new(ListPodSandboxStatsResponse { stats }))
    }

    /// UpdateRuntimeConfig updates the runtime configuration based on the given request.
    async fn update_runtime_config(
        &self,
        request: tonic::Request<UpdateRuntimeConfigRequest>,
    ) -> Result<tonic::Response<UpdateRuntimeConfigResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::update_runtime_config");

        let req = request.into_inner();
        for client in self.clients.values() {
            let mut client = client.clone();
            let _ = client
                .update_runtime_config(tonic::Request::new(req.clone()))
                .await?;
        }

        Ok(tonic::Response::new(UpdateRuntimeConfigResponse {}))
    }
    /// Status returns the status of the runtime.
    async fn status(
        &self,
        request: tonic::Request<StatusRequest>,
    ) -> Result<tonic::Response<StatusResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::status");
        let mut client = self.get_default_client()?;
        client.status(request).await
    }
    /// CheckpointContainer checkpoints a container
    async fn checkpoint_container(
        &self,
        request: tonic::Request<CheckpointContainerRequest>,
    ) -> Result<tonic::Response<CheckpointContainerResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::checkpoint_container");

        let req = request.into_inner();
        let container = self.storage.get_container(&req.container_id).await?;
        let mut client = self.get_client(&container.runtime)?;

        client.checkpoint_container(tonic::Request::new(req)).await
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
        let mut descriptors = vec![];

        let req = request.into_inner();
        for client in self.clients.values() {
            let mut client = client.clone();
            let d = client
                .list_metric_descriptors(tonic::Request::new(req.clone()))
                .await?
                .into_inner();
            descriptors.extend(d.descriptors);
        }

        Ok(tonic::Response::new(ListMetricDescriptorsResponse {
            descriptors,
        }))
    }

    /// ListPodSandboxMetrics gets pod sandbox metrics from CRI Runtime
    async fn list_pod_sandbox_metrics(
        &self,
        request: tonic::Request<ListPodSandboxMetricsRequest>,
    ) -> Result<tonic::Response<ListPodSandboxMetricsResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::list_pod_sandbox_metrics");

        let mut pod_metrics = vec![];

        let req = request.into_inner();
        for client in self.clients.values() {
            let mut client = client.clone();
            let d = client
                .list_pod_sandbox_metrics(tonic::Request::new(req.clone()))
                .await?
                .into_inner();
            pod_metrics.extend(d.pod_metrics);
        }

        Ok(tonic::Response::new(ListPodSandboxMetricsResponse {
            pod_metrics,
        }))
    }

    async fn runtime_config(
        &self,
        request: tonic::Request<RuntimeConfigRequest>,
    ) -> Result<tonic::Response<RuntimeConfigResponse>, tonic::Status> {
        trace_fn!("RuntimeShim::runtime_config");
        let mut client = self.get_default_client()?;

        client.runtime_config(request).await
    }
}
