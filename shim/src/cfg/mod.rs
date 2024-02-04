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
use clap::Parser;

// use crate::cri;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Options {
    // The address of Unix socket for Chariot shim.
    // #[arg(short, long, default_value=cri::DEFAULT_UNIX_SOCKET)]
    // pub address: String,

    /// The address of CRI server in XPU.
    #[arg(short, long)]
    pub xpu_address: String,
}
