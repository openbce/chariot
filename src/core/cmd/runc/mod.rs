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
    mount::{mount, MsFlags},
    sched::CloneFlags,
    sys::wait::wait,
    unistd::{chdir, execv, getpid, pivot_root},
};

use flate2::read::GzDecoder;
use oci_spec::image::ImageManifest;

use crate::cfg;
use chariot::apis::{ChariotResult, Container};

pub async fn run(cxt: cfg::Context, file: String) -> ChariotResult<()> {
    let yaml = fs::read_to_string(file)?;
    let container: Container = serde_yaml::from_str(&yaml)?;

    let mut stack = [0u8; 1024 * 1024];
    let flags = CloneFlags::empty()
        .union(CloneFlags::CLONE_NEWUSER)
        .union(CloneFlags::CLONE_NEWNET)
        .union(CloneFlags::CLONE_NEWPID)
        .union(CloneFlags::CLONE_NEWNS);

    tracing::debug!("Parent pid is <{}>.", getpid());

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
    tracing::debug!("The container <{}> was exited", status.pid().unwrap());

    Ok(())
}

fn run_container(cxt: cfg::Context, container: Container) -> ChariotResult<()> {
    tracing::debug!("Run sandbox <{}> as <{}>.", container.name, getpid());

    // TODO: get the CHARIOT_HOME from configure file.
    let manifest_path = format!("{}/{}/manifest.json", cxt.image_dir(), container.image);
    tracing::debug!("Loading image manifest <{}>.", manifest_path);
    let image_manifest = ImageManifest::from_file(manifest_path)?;

    let rootfs = format!("{}/{}", cxt.container_dir(), container.name);
    let oldfs = format!("{}/.pivot_root", rootfs);

    for layer in image_manifest.layers() {
        // TODO: detect mediaType and select unpack tools accordingly.
        let layer_path = format!(
            "{}/{}/{}",
            cxt.image_dir(),
            container.image,
            layer.digest().digest()
        );
        let tar_gz = fs::File::open(layer_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(&rootfs)?;
    }

    // Change to rootfs.
    fs::create_dir_all(&oldfs)?;
    tracing::debug!("Change root to <{}>, and the old fs is <{}>", rootfs, oldfs);

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

    let _ = pivot_root(rootfs.as_str(), oldfs.as_str())?;

    // TODO: unmout the old fs
    // umount(oldfs)

    // Change working directory to '/'.
    chdir("/")?;
    tracing::debug!(
        "Change working directory to </>, and execv <{}>",
        container.entrypoint
    );

    // execute `container entrypoint`
    let cmd = CString::new(container.entrypoint.as_bytes())?;
    let _ = execv(cmd.as_c_str(), &[cmd.as_c_str()]);

    Ok(())
}
