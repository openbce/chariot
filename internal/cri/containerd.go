package cri

import (
	"context"
	"time"

	runtimeapi "k8s.io/cri-api/pkg/apis/runtime/v1"
)

type XpuShim struct {
}

// ListImages lists the existing images.
func (s *XpuShim) ListImages(ctx context.Context, filter *runtimeapi.ImageFilter) ([]*runtimeapi.Image, error) {
	return nil, nil
}

// ImageStatus returns the status of the image.
func (s *XpuShim) ImageStatus(ctx context.Context, image *runtimeapi.ImageSpec, verbose bool) (*runtimeapi.ImageStatusResponse, error) {
	return nil, nil
}

// PullImage pulls an image with the authentication config.
func (s *XpuShim) PullImage(ctx context.Context, image *runtimeapi.ImageSpec, auth *runtimeapi.AuthConfig, podSandboxConfig *runtimeapi.PodSandboxConfig) (string, error) {
	return "", nil
}

// RemoveImage removes the image.
func (s *XpuShim) RemoveImage(ctx context.Context, image *runtimeapi.ImageSpec) error {
	return nil
}

// ImageFsInfo returns information of the filesystem(s) used to store the read-only layers and the writeable layer.
func (s *XpuShim) ImageFsInfo(ctx context.Context) (*runtimeapi.ImageFsInfoResponse, error) {
	return nil, nil
}

// UpdateRuntimeConfig updates runtime configuration if specified
func (s *XpuShim) UpdateRuntimeConfig(ctx context.Context, runtimeConfig *runtimeapi.RuntimeConfig) error {
	return nil
}

// Status returns the status of the runtime.
func (s *XpuShim) Status(ctx context.Context, verbose bool) (*runtimeapi.StatusResponse, error) {
	return nil, nil
}

// RuntimeConfig returns the configuration information of the runtime.
func (s *XpuShim) RuntimeConfig(ctx context.Context) (*runtimeapi.RuntimeConfigResponse, error) {
	return nil, nil
}

// ContainerStats returns stats of the container. If the container does not
// exist, the call returns an error.
func (s *XpuShim) ContainerStats(ctx context.Context, containerID string) (*runtimeapi.ContainerStats, error) {
	return nil, nil
}

// ListContainerStats returns stats of all running containers.
func (s *XpuShim) ListContainerStats(ctx context.Context, filter *runtimeapi.ContainerStatsFilter) ([]*runtimeapi.ContainerStats, error) {
	return nil, nil
}

// PodSandboxStats returns stats of the pod. If the pod does not
// exist, the call returns an error.
func (s *XpuShim) PodSandboxStats(ctx context.Context, podSandboxID string) (*runtimeapi.PodSandboxStats, error) {
	return nil, nil
}

// ListPodSandboxStats returns stats of all running pods.
func (s *XpuShim) ListPodSandboxStats(ctx context.Context, filter *runtimeapi.PodSandboxStatsFilter) ([]*runtimeapi.PodSandboxStats, error) {
	return nil, nil
}

// ListMetricDescriptors gets the descriptors for the metrics that will be returned in ListPodSandboxMetrics.
func (s *XpuShim) ListMetricDescriptors(ctx context.Context) ([]*runtimeapi.MetricDescriptor, error) {
	return nil, nil
}

// ListPodSandboxMetrics returns metrics of all running pods.
func (s *XpuShim) ListPodSandboxMetrics(ctx context.Context) ([]*runtimeapi.PodSandboxMetrics, error) {
	return nil, nil
}

// RunPodSandbox creates and starts a pod-level sandbox. Runtimes should ensure
// the sandbox is in ready state.
func (s *XpuShim) RunPodSandbox(ctx context.Context, config *runtimeapi.PodSandboxConfig, runtimeHandler string) (string, error) {
	return "", nil
}

// StopPodSandbox stops the sandbox. If there are any running containers in the
// sandbox, they should be force terminated.
func (s *XpuShim) StopPodSandbox(pctx context.Context, odSandboxID string) error {
	return nil
}

// RemovePodSandbox removes the sandbox. If there are running containers in the
// sandbox, they should be forcibly removed.
func (s *XpuShim) RemovePodSandbox(ctx context.Context, podSandboxID string) error {
	return nil
}

// PodSandboxStatus returns the Status of the PodSandbox.
func (s *XpuShim) PodSandboxStatus(ctx context.Context, podSandboxID string, verbose bool) (*runtimeapi.PodSandboxStatusResponse, error) {
	return nil, nil
}

// ListPodSandbox returns a list of Sandbox.
func (s *XpuShim) ListPodSandbox(ctx context.Context, filter *runtimeapi.PodSandboxFilter) ([]*runtimeapi.PodSandbox, error) {
	return nil, nil
}

// PortForward prepares a streaming endpoint to forward ports from a PodSandbox, and returns the address.
func (s *XpuShim) PortForward(context.Context, *runtimeapi.PortForwardRequest) (*runtimeapi.PortForwardResponse, error) {
	return nil, nil
}

// CreateContainer creates a new container in specified PodSandbox.
func (s *XpuShim) CreateContainer(ctx context.Context, podSandboxID string, config *runtimeapi.ContainerConfig, sandboxConfig *runtimeapi.PodSandboxConfig) (string, error) {
	return "", nil
}

// StartContainer starts the container.
func (s *XpuShim) StartContainer(ctx context.Context, containerID string) error {
	return nil
}

// StopContainer stops a running container with a grace period (i.e., timeout).
func (s *XpuShim) StopContainer(ctx context.Context, containerID string, timeout int64) error {
	return nil
}

// RemoveContainer removes the container.
func (s *XpuShim) RemoveContainer(ctx context.Context, containerID string) error {
	return nil
}

// ListContainers lists all containers by filters.
func (s *XpuShim) ListContainers(ctx context.Context, filter *runtimeapi.ContainerFilter) ([]*runtimeapi.Container, error) {
	return nil, nil
}

// ContainerStatus returns the status of the container.
func (s *XpuShim) ContainerStatus(ctx context.Context, containerID string, verbose bool) (*runtimeapi.ContainerStatusResponse, error) {
	return nil, nil
}

// UpdateContainerResources updates ContainerConfig of the container synchronously.
// If runtime fails to transactionally update the requested resources, an error is returned.
func (s *XpuShim) UpdateContainerResources(ctx context.Context, containerID string, resources *runtimeapi.ContainerResources) error {
	return nil
}

// ExecSync executes a command in the container, and returns the stdout output.
// If command exits with a non-zero exit code, an error is returned.
func (s *XpuShim) ExecSync(ctx context.Context, containerID string, cmd []string, timeout time.Duration) (stdout []byte, stderr []byte, err error) {
	return nil, nil, nil
}

// Exec prepares a streaming endpoint to execute a command in the container, and returns the address.
func (s *XpuShim) Exec(context.Context, *runtimeapi.ExecRequest) (*runtimeapi.ExecResponse, error) {
	return nil, nil
}

// Attach prepares a streaming endpoint to attach to a running container, and returns the address.
func (s *XpuShim) Attach(ctx context.Context, req *runtimeapi.AttachRequest) (*runtimeapi.AttachResponse, error) {
	return nil, nil
}

// ReopenContainerLog asks runtime to reopen the stdout/stderr log file
// for the container. If it returns error, new container log file MUST NOT
// be created.
func (s *XpuShim) ReopenContainerLog(ctx context.Context, ContainerID string) error {
	return nil
}

// CheckpointContainer checkpoints a container
func (s *XpuShim) CheckpointContainer(ctx context.Context, options *runtimeapi.CheckpointContainerRequest) error {
	return nil
}

// GetContainerEvents gets container events from the CRI runtime
func (s *XpuShim) GetContainerEvents(containerEventsCh chan *runtimeapi.ContainerEventResponse) error {
	return nil
}

// Version returns the runtime name, runtime version and runtime API version
func (s *XpuShim) Version(ctx context.Context, apiVersion string) (*runtimeapi.VersionResponse, error) {
	return nil, nil
}
