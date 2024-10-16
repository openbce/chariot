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

use std::ffi::{CStr, CString};
use std::fs;

use nix::{
    fcntl::{open, OFlag},
    libc,
    mount::{mount, umount2, MntFlags, MsFlags},
    sched::CloneFlags,
    sys::{
        stat::Mode,
        wait::{wait, WaitStatus},
    },
    unistd::{chdir, dup2, execve, getpid, getuid, pivot_root},
};

use flate2::read::GzDecoder;
use oci_spec::image::ImageManifest;

use crate::cfg;
use chariot::apis::{ChariotResult, Container};

pub async fn run(cxt: cfg::Context, file: String) -> ChariotResult<()> {
    let yaml = fs::read_to_string(file)?;
    let container: Container = serde_yaml::from_str(&yaml)?;

    tracing::debug!("Parent pid is <{}>.", getpid());

    let manifest_path = cxt.image_manifest(&container.image);
    tracing::debug!("Loading image manifest <{}>.", manifest_path);
    let image_manifest = ImageManifest::from_file(manifest_path)?;

    let rootfs = cxt.container_rootfs(&container.name);
    let imagefs = cxt.image_dir(&container.name);

    for layer in image_manifest.layers() {
        // TODO: detect mediaType and select unpack tools accordingly.
        let layer_path = format!("{}/{}", imagefs, layer.digest().digest());
        let tar_gz = fs::File::open(layer_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(&rootfs)?;
    }

    let mut stack = [0u8; 1024 * 1024];
    let flags = CloneFlags::empty()
        .union(CloneFlags::CLONE_NEWUSER)
        .union(CloneFlags::CLONE_NEWNET)
        .union(CloneFlags::CLONE_NEWPID)
        .union(CloneFlags::CLONE_NEWNS);

    let pid = unsafe {
        nix::sched::clone(
            Box::new(|| match run_container(cxt.clone(), container.clone()) {
                Ok(()) => 0,
                Err(e) => {
                    tracing::error!("Failed to run container: {e}");
                    -1
                }
            }),
            &mut stack,
            flags,
            // The SIGCHLD signal is required for wait/waitpid;
            // otherwise, ECHILD will be reported.
            Some(libc::SIGCHLD),
        )?
    };

    tracing::debug!("Waiting for the child process <{pid}> to finish.");
    let status = wait()?;
    match status {
        WaitStatus::Exited(pid, rc) => {
            if rc == 0 {
                tracing::info!("The container <{}> was exited successfully.", pid);
            } else {
                tracing::error!("The container <{}> was exited in <{}>.", pid, rc);
            }
        }
        WaitStatus::Signaled(pid, sig, _) => {
            tracing::info!("The container <{}> got a signal <{}>.", pid, sig);
        }
        WaitStatus::Stopped(pid, sig) => {
            tracing::info!("The container <{}> was stopped by signal <{}>.", pid, sig);
        }
        _ => {}
    }

    Ok(())
}

fn run_container(cxt: cfg::Context, container: Container) -> ChariotResult<()> {
    tracing::debug!(
        "Run sandbox <{}> in <{}> as <{}>.",
        container.name,
        getpid(),
        getuid()
    );

    // Re-direct the stdout/stderr to log file.
    let logfile = cxt.container_log(&container.name);
    let log = open(
        logfile.as_str(),
        OFlag::empty().union(OFlag::O_CREAT).union(OFlag::O_RDWR),
        Mode::from_bits(0o755).unwrap(),
    )?;

    // Change the root of container by pivot_root.
    change_root(cxt.clone(), container.clone())?;

    tracing::debug!("Redirect container stdout/stderr to log file.");
    dup2(log, 1)?;
    dup2(log, 2)?;

    // execute `container entrypoint`
    let cmd = CString::new(container.entrypoint.as_bytes())?;
    execve::<&CStr, &CStr>(cmd.as_c_str(), &[cmd.as_c_str()], &[])?;

    Ok(())
}

fn change_root(cxt: cfg::Context, container: Container) -> ChariotResult<()> {
    // Create rootfs.
    let rootfs = cxt.container_rootfs(&container.name);
    tracing::debug!("Change root to <{}>", rootfs);

    // Ensure that 'new_root' and its parent mount don't have
    // shared propagation (which would cause pivot_root() to
    // return an error), and prevent propagation of mount
    // events to the initial mount namespace.
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::empty()
            .union(MsFlags::MS_REC)
            .union(MsFlags::MS_PRIVATE),
        None::<&str>,
    )?;

    // Ensure that 'new_root' is a mount point.
    mount(
        Some(rootfs.as_str()),
        rootfs.as_str(),
        None::<&str>,
        MsFlags::MS_BIND,
        None::<&str>,
    )?;

    pivot_root(rootfs.as_str(), rootfs.as_str())?;

    tracing::debug!("Detach the rootfs from parent system.");
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_SLAVE | MsFlags::MS_REC,
        None::<&str>,
    )?;
    umount2("/", MntFlags::MNT_DETACH)?;

    // Change working directory to '/'.
    chdir("/")?;

    Ok(())
}
