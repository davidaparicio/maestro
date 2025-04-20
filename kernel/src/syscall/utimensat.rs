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

//! The `utimensat` system call allows to change the timestamps of a file.

use super::util::at;
use crate::{
	file::{
		fd::FileDescriptorTable,
		fs::StatSet,
		vfs,
		vfs::{ResolutionSettings, Resolved},
	},
	process::{
		mem_space::copy::{SyscallPtr, SyscallString},
		Process,
	},
	sync::mutex::Mutex,
	syscall::Args,
	time,
	time::{
		clock,
		clock::{current_time_ns, Clock},
		unit::{TimeUnit, Timespec},
	},
	tty::vga::DEFAULT_COLOR,
};
use core::ffi::c_int;
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn utimensat(
	Args((dirfd, pathname, times, flags)): Args<(
		c_int,
		SyscallString,
		SyscallPtr<[Timespec; 2]>,
		c_int,
	)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let pathname = pathname
		.copy_from_user()?
		.map(PathBuf::try_from)
		.transpose()?;
	let (atime, mtime) = times
		.copy_from_user()?
		.map(|[atime, mtime]| (atime.to_nano(), mtime.to_nano()))
		.unwrap_or_else(|| {
			let ts = current_time_ns(Clock::Monotonic);
			(ts, ts)
		});
	// Get file
	let Resolved::Found(file) = at::get_file(&fds.lock(), rs, dirfd, pathname.as_deref(), flags)?
	else {
		return Err(errno!(ENOENT));
	};
	// Update timestamps
	vfs::set_stat(
		file.node(),
		&StatSet {
			atime: Some(atime / 1_000_000_000),
			mtime: Some(mtime / 1_000_000_000),
			..Default::default()
		},
	)?;
	Ok(0)
}
