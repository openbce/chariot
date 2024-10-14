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

use serde::{Deserialize, Serialize};

use self::engine::EnginePtr;
use chariot::apis::ChariotError;

mod engine;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Container {
    pub id: String,
    pub sandbox: String,

    pub runtime: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Sandbox {
    pub id: String,

    pub runtime: String,
}

pub struct Storage {
    engine: EnginePtr,
}

pub fn new(p: &str) -> Result<Storage, ChariotError> {
    Ok(Storage {
        engine: Arc::new(engine::fs::FsEngine::new(p)?),
    })
}

impl Storage {
    pub async fn persist_container(&self, c: &Container) -> Result<(), ChariotError> {
        self.engine.persist_container(c).await
    }

    pub async fn get_container(&self, id: &str) -> Result<Container, ChariotError> {
        self.engine.get_container(id).await
    }

    pub async fn list_containers(&self) -> Result<Vec<Container>, ChariotError> {
        self.engine.list_containers().await
    }

    pub async fn remove_container(&self, id: &str) -> Result<(), ChariotError> {
        self.engine.remove_container(id).await
    }

    pub async fn persist_sandbox(&self, s: &Sandbox) -> Result<(), ChariotError> {
        self.engine.persist_sandbox(s).await
    }

    pub async fn get_sandbox(&self, id: &str) -> Result<Sandbox, ChariotError> {
        self.engine.get_sandbox(id).await
    }

    pub async fn list_sandboxes(&self) -> Result<Vec<Sandbox>, ChariotError> {
        self.engine.list_sandboxes().await
    }

    pub async fn remove_sandbox(&self, id: &str) -> Result<(), ChariotError> {
        self.engine.remove_sandbox(id).await
    }
}
