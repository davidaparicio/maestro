/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! The `bind` system call binds a name to a socket.

use crate::{
	file::{fd::FileDescriptorTable, socket::Socket},
	memory::user::UserSlice,
	process::Process,
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{any::Any, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn bind(
	Args((sockfd, addr, addrlen)): Args<(c_int, *mut u8, isize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if addrlen < 0 {
		return Err(errno!(EINVAL));
	}
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	let addr = UserSlice::from_user(addr, addrlen as _)?;
	let addr = addr.copy_from_user_vec(0)?.ok_or_else(|| errno!(EFAULT))?;
	sock.bind(&addr)?;
	Ok(0)
}
