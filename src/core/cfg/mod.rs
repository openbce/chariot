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

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Commands {
    /// Run a container directly.
    Runc {
        /// The yaml file of container to run.
        #[arg(short, long)]
        file: String,
    },

    /// Run a Pod directly.
    Runp,

    /// Start a CRI service.
    Start,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct ChariotOptions {
    /// The work directory of Chariot.
    #[arg(short, long, default_value = "/opt/chariot")]
    pub work_dir: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Context {
    pub work_dir: String,
}

impl From<&ChariotOptions> for Context {
    fn from(o: &ChariotOptions) -> Self {
        Self {
            work_dir: o.work_dir.clone(),
        }
    }
}

impl Context {
    pub fn work_dir(&self) -> String {
        self.work_dir.clone()
    }

    pub fn image_dir(&self) -> String {
        format!("{}/images", self.work_dir)
    }

    pub fn container_dir(&self) -> String {
        format!("{}/containers", self.work_dir)
    }
}
