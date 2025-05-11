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

//! The `readlink` syscall allows to read the target of a symbolic link.

use crate::{
	file::{FileType, vfs, vfs::ResolutionSettings},
	memory::user::{UserSlice, UserString},
	process::Process,
	syscall::Args,
};
use utils::{
	collections::{path::PathBuf, vec::Vec},
	errno,
	errno::{EResult, Errno},
	vec,
};

pub fn readlink(
	Args((pathname, buf, bufsiz)): Args<(UserString, *mut u8, usize)>,
) -> EResult<usize> {
	let proc = Process::current();
	// Get file
	let path = pathname.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	let rs = ResolutionSettings::for_process(&proc, false);
	let ent = vfs::get_file_from_path(&path, &rs)?;
	// Validation
	if ent.get_type()? != FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Read link
	let buf = UserSlice::from_user(buf, bufsiz)?;
	let node = ent.node();
	let len = node.node_ops.readlink(node, buf)?;
	Ok(len as _)
}
