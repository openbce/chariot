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

use std::sync::Arc;

use super::{Container, Sandbox};
use chariot_sys::ChariotError;

pub mod fs;

pub type EnginePtr = Arc<dyn Engine>;

#[async_trait::async_trait]
pub trait Engine: Send + Sync + 'static {
    async fn persist_container(&self, c: &Container) -> Result<(), ChariotError>;
    async fn get_container(&self, id: &str) -> Result<Container, ChariotError>;
    async fn list_containers(&self) -> Result<Vec<Container>, ChariotError>;
    async fn remove_container(&self, id: &str) -> Result<(), ChariotError>;

    async fn persist_sandbox(&self, s: &Sandbox) -> Result<(), ChariotError>;
    async fn get_sandbox(&self, id: &str) -> Result<Sandbox, ChariotError>;
    async fn list_sandboxes(&self) -> Result<Vec<Sandbox>, ChariotError>;
    async fn remove_sandbox(&self, id: &str) -> Result<(), ChariotError>;
}
