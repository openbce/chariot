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

use std::fs;
use std::io::prelude::*;

use super::Engine;

use super::{Container, Sandbox};
use sys::ChariotError;

pub struct FsEngine {
    sandbox_path: String,
    container_path: String,
}

impl FsEngine {
    pub fn new(p: &str) -> Result<Self, ChariotError> {
        let sandbox_path = format!("{}/sandboxes", p);
        let container_path = format!("{}/containers", p);

        fs::create_dir_all(&sandbox_path)?;
        fs::create_dir_all(&container_path)?;

        Ok(FsEngine {
            sandbox_path,
            container_path,
        })
    }
}

#[async_trait::async_trait]
impl Engine for FsEngine {
    async fn persist_container(&self, c: &Container) -> Result<(), ChariotError> {
        let path = format!("{}/{}", self.container_path, c.id);
        let mut file = fs::File::create(path)?;
        let data = serde_json::to_string(c)?;
        file.write_all(data.as_bytes())?;

        Ok(())
    }

    async fn get_container(&self, id: &str) -> Result<Container, ChariotError> {
        let path = format!("{}/{}", self.container_path, id);
        let data = fs::read_to_string(path)?;

        Ok(serde_json::from_str(data.as_str())?)
    }

    async fn list_containers(&self) -> Result<Vec<Container>, ChariotError> {
        let mut containers = vec![];
        for entry in fs::read_dir(&self.container_path)? {
            let data = fs::read_to_string(entry?.path())?;
            containers.push(serde_json::from_str(data.as_str())?);
        }

        Ok(containers)
    }

    async fn remove_container(&self, id: &str) -> Result<(), ChariotError> {
        let path = format!("{}/{}", self.container_path, id);
        fs::remove_file(path).map_err(ChariotError::from)
    }

    async fn persist_sandbox(&self, s: &Sandbox) -> Result<(), ChariotError> {
        let path = format!("{}/{}", self.sandbox_path, s.id);
        let mut file = fs::File::create(path)?;
        let data = serde_json::to_string(s)?;
        file.write_all(data.as_bytes())?;

        Ok(())
    }

    async fn get_sandbox(&self, id: &str) -> Result<Sandbox, ChariotError> {
        let path = format!("{}/{}", self.sandbox_path, id);
        let data = fs::read_to_string(path)?;

        Ok(serde_json::from_str(data.as_str())?)
    }

    async fn list_sandboxes(&self) -> Result<Vec<Sandbox>, ChariotError> {
        let mut sandboxes = vec![];
        for entry in fs::read_dir(&self.sandbox_path)? {
            let data = fs::read_to_string(entry?.path())?;
            sandboxes.push(serde_json::from_str(data.as_str())?);
        }

        Ok(sandboxes)
    }

    async fn remove_sandbox(&self, id: &str) -> Result<(), ChariotError> {
        let path = format!("{}/{}", self.sandbox_path, id);
        fs::remove_file(path).map_err(ChariotError::from)
    }
}
