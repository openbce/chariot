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

use std::ffi::CString;
use std::fs;

use nix::{
    libc,
    sched::CloneFlags,
    sys::wait::wait,
    unistd::{execv, getpid, pivot_root},
};

use chariot::apis::{ChariotResult, Sandbox};

pub async fn run(file: String) -> ChariotResult<()> {
    let yaml = fs::read_to_string(file)?;
    let sandbox: Sandbox = serde_yaml::from_str(&yaml)?;

    let mut stack = [0u8; 1024 * 1024];
    let flags = CloneFlags::empty()
        .union(CloneFlags::CLONE_NEWUSER)
        .union(CloneFlags::CLONE_NEWNET)
        .union(CloneFlags::CLONE_NEWPID)
        .union(CloneFlags::CLONE_NEWNS);

    tracing::debug!("Parent pid is <{}>.", getpid());

    let pid = unsafe {
        nix::sched::clone(
            Box::new(|| run_standbox(sandbox.clone())),
            &mut stack,
            flags,
            // The SIGCHLD signal is required for wait/waitpid;
            // otherwise, ECHILD will be reported.
            Some(libc::SIGCHLD),
        )?
    };

    tracing::debug!("Waiting for the child process <{pid}> to finish.");
    let status = wait()?;
    tracing::info!("The container <{}> was exited", status.pid().unwrap());

    Ok(())
}

fn run_standbox(sandbox: Sandbox) -> isize {
    tracing::debug!("Child pid is <{}>.", getpid());
    // TODO: setup environment for the container, e.g. pivot_root

    // let _ = pivot_root(new_root, put_old);
    let cmd = CString::new(sandbox.entrypoint.as_bytes()).unwrap();

    // execute `Sandbox entrypoint`
    let _ = execv(cmd.as_c_str(), &[cmd.as_c_str()]);

    0
}
