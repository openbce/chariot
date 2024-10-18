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
use std::io::Write;

// use nix::mount::{mount, MsFlags};
use nix::unistd::Pid;

use chariot::apis::{ChariotResult, Container};

use crate::cfg::Context;

pub fn setup(cxt: &Context) -> ChariotResult<()> {
    if !fs::exists(cxt.cgroup_root())? {
        fs::create_dir_all(cxt.cgroup_root())?;
    }

    // TODO: Moutn cgroup to Chariot home and setup sub-controllers accordingly.

    Ok(())
}

pub fn add_container(cxt: &Context, container: &Container, pid: Pid) -> ChariotResult<()> {
    let root = cxt.cgroup_dir(&container.name);
    if !fs::exists(&root)? {
        fs::create_dir(&root)?;
    }

    // TODO: setup cpu, cpuset and mem.

    let procs_path = format!("{root}/cgroup.procs");
    let mut procs = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .open(&procs_path)?;

    let pid = format!("{}", pid);
    tracing::debug!("Add process <{pid}> into cgroup <{procs_path}>");

    procs.write_all(pid.as_bytes())?;

    Ok(())
}

pub fn remove_container(cxt: &Context, container: &Container) -> ChariotResult<()> {
    fs::remove_dir(cxt.cgroup_dir(&container.name))?;

    Ok(())
}
