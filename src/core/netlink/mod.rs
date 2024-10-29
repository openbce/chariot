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

use std::os::fd::AsRawFd;

use chariot::apis::{ChariotError, ChariotResult};

use nix::sys::socket::{
    bind, socket, AddressFamily, NetlinkAddr, SockFlag, SockProtocol, SockType,
};
use nix::unistd::getpid;

pub fn add_route() -> ChariotResult<()> {
    let fd = socket(
        AddressFamily::Netlink,
        SockType::Raw,
        SockFlag::empty(),
        SockProtocol::NetlinkRoute,
    )?;

    let pid = {
        let pid = getpid().as_raw();
        if pid < 0 {
            Err(Box::new(ChariotError::Internal("invalid pid".to_string())))
        } else {
            Ok(pid)
        }
    }?;

    let addr = NetlinkAddr::new(pid as u32, 0);

    bind(fd.as_raw_fd(), &addr)?;

    Ok(())
}

pub fn delete_route() -> ChariotResult<()> {
    todo!()
}

pub fn add_snat() -> ChariotResult<()> {
    todo!()
}

pub fn add_dnat() -> ChariotResult<()> {
    todo!()
}
