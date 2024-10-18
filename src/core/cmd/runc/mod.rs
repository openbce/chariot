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
use std::io::Write;
use std::{thread, time};

use flate2::read::GzDecoder;
use nix::{
    // fcntl::{open, OFlag},
    libc,
    mount::{mount, umount2, MntFlags, MsFlags},
    sched::CloneFlags,
    sys::{
        // stat::Mode,
        wait::{wait, WaitStatus},
    },
    unistd::{chdir, execve, getgid, getpid, getuid, pivot_root, Gid, Pid, Uid},
};
use oci_spec::image::ImageManifest;

use chariot::apis::{ChariotResult, Container};

use crate::cfg;
use crate::cgroup;

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

    tracing::debug!("Setup and run container.");
    let mut stack = [0u8; 1024 * 1024];
    let flags = CloneFlags::empty()
        .union(CloneFlags::CLONE_NEWUSER)
        .union(CloneFlags::CLONE_NEWNET)
        .union(CloneFlags::CLONE_NEWUTS)
        .union(CloneFlags::CLONE_NEWIPC)
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

    cgroup::add_container(&cxt, &container, pid)?;

    // Setup user/group mapping for the container.
    setup_user_mapping(pid)?;

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

    cgroup::remove_container(&cxt, &container)?;

    Ok(())
}

fn setup_user_mapping(pid: Pid) -> ChariotResult<()> {
    tracing::debug!(
        "Run chariot in <{}> as <{}:{}>.",
        getpid(),
        getuid(),
        getgid()
    );

    // Setup user mapping.
    let uid_map = format!("/proc/{}/uid_map", pid);
    let mapping = format!("0 {} 1", getuid());
    tracing::debug!("setup user mapping: {uid_map}, {mapping}");

    let mut file = fs::File::create(uid_map)?;
    file.write_all(mapping.as_bytes())?;

    // Disable setgroups for unpriviledge user;
    // otherwise, we can not setup group mapping.
    let setgroups = format!("/proc/{}/setgroups", pid);
    let mut file = fs::File::create(setgroups)?;
    file.write_all(b"deny")?;

    // Setup group mapping.
    let gid_map = format!("/proc/{}/gid_map", pid);
    let mapping = format!("0 {} 1", getgid());
    tracing::debug!("setup group mapping: {gid_map}, {mapping}");

    let mut file = fs::File::create(gid_map)?;
    file.write_all(mapping.as_bytes())?;

    Ok(())
}

fn run_container(cxt: cfg::Context, container: Container) -> ChariotResult<()> {
    // Waiting for user mapping ready.
    let ten_millis = time::Duration::from_millis(10);
    loop {
        if getgid() == Gid::from(0) && getuid() == Uid::from(0) {
            break;
        }
        thread::sleep(ten_millis);
    }

    // Start to run container
    tracing::debug!(
        "Run container <{}> in <{}> as <{}:{}>.",
        container.name,
        getpid(),
        getuid(),
        getgid(),
    );

    // Re-direct the stdout/stderr to log file.
    // let logfile = cxt.container_log(&container.name);
    // let log = open(
    //     logfile.as_str(),
    //     OFlag::empty().union(OFlag::O_CREAT).union(OFlag::O_RDWR),
    //     Mode::from_bits(0o755).unwrap(),
    // )?;

    // Change the root of container by pivot_root.
    change_root(cxt.clone(), container.clone())?;

    let cmd = CString::new(container.entrypoint[0].as_bytes())?;
    let args = container
        .entrypoint
        .iter()
        .filter_map(|c| CString::new(c.as_bytes()).ok())
        .collect::<Vec<_>>();

    // execute `container entrypoint`
    tracing::debug!("Redirect container stdout/stderr to log file, and execve the entrypoint.");
    // dup2(log, 1)?;
    // dup2(log, 2)?;
    execve::<CString, CString>(&cmd, args.as_slice(), &[])?;

    Ok(())
}

fn setup_fstab(_: cfg::Context, _: Container) -> ChariotResult<()> {
    tracing::debug!("Try to mout /proc by <{}>", getuid());
    mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::empty()
            .union(MsFlags::MS_NOEXEC)
            .union(MsFlags::MS_NODEV)
            .union(MsFlags::MS_NOSUID),
        None::<&str>,
    )?;
    tracing::debug!("Try to mout /dev by <{}>", getuid());
    mount(
        Some("tmpfs"),
        "/dev",
        Some("tmpfs"),
        MsFlags::empty()
            .union(MsFlags::MS_STRICTATIME)
            .union(MsFlags::MS_NOSUID),
        Some("mode=0755"),
    )?;

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

    // Setup fstab, e.g. /proc, /dev, before unmount parent FS.
    setup_fstab(cxt.clone(), container.clone())?;

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
