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

//! The `getsockopt` system call gets an option on a socket.

use crate::{
	file::{buffer, buffer::socket::Socket},
	process::{mem_space::copy::SyscallSlice, Process},
	syscall::Args,
};
use core::{any::Any, cmp::min, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn getsockopt(
	Args((sockfd, level, optname, optval, optlen)): Args<(
		c_int,
		c_int,
		c_int,
		SyscallSlice<u8>,
		usize,
	)>,
) -> EResult<usize> {
	let proc_mutex = Process::current();
	let proc = proc_mutex.lock();

	// Get socket
	let fds_mutex = proc.file_descriptors.as_ref().unwrap();
	let fds = fds_mutex.lock();
	let fd = fds.get_fd(sockfd)?;
	let open_file_mutex = fd.get_open_file();
	let open_file = open_file_mutex.lock();
	let loc = open_file.get_location();
	let sock_mutex = buffer::get(loc).ok_or_else(|| errno!(ENOENT))?;
	let mut sock = sock_mutex.lock();
	let sock = (&mut *sock as &mut dyn Any)
		.downcast_mut::<Socket>()
		.ok_or_else(|| errno!(ENOTSOCK))?;
	let val = sock.get_opt(level, optname)?;
	// Write back
	let len = min(val.len(), optlen);
	optval.copy_to_user(&val[..len])?;
	Ok(len as _)
}
