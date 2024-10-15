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
    unistd::{execv, getpid},
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

    for layer in image_manifest.layers() {
        // TODO: detect mediaType and select unpack tools accordingly.
        let layer_path = format!("{}/{}/{}", cxt.image_dir(), container.image, layer.digest().digest());
        let tar_gz = fs::File::open(layer_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(format!("{}/{}", cxt.container_dir(), container.name))?;
    }

    // TODO: setup environment for the container, e.g. pivot_root
    // let _ = pivot_root(new_root, put_old);
    let cmd = CString::new(container.entrypoint.as_bytes())?;

    // execute `container entrypoint`
    let _ = execv(cmd.as_c_str(), &[cmd.as_c_str()]);

    Ok(())
}
