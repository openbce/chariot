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

use std::error::Error;
use std::io;

use serde::{Deserialize, Serialize};

pub type ChariotResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Sandbox {
    pub name: String,
    pub image: String,
    pub entrypoint: Vec<String>,
    pub containers: Vec<Container>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Container {
    pub name: String,
    pub image: String,
    pub entrypoint: Vec<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum ChariotError {
    #[error("{0}")]
    Network(String),
    #[error("{0}")]
    Cri(String),
    #[error("{0}")]
    IO(String),
    #[error("{0}")]
    Json(String),
}

impl From<io::Error> for ChariotError {
    fn from(e: io::Error) -> Self {
        ChariotError::IO(e.to_string())
    }
}

impl From<serde_json::Error> for ChariotError {
    fn from(e: serde_json::Error) -> Self {
        ChariotError::Json(e.to_string())
    }
}

impl From<ChariotError> for tonic::Status {
    fn from(e: ChariotError) -> Self {
        tonic::Status::internal(e.to_string())
    }
}
