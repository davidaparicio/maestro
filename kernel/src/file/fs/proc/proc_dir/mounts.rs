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

//! Implementation of the `mounts` node which allows to get the list of mountpoint.

use crate::{
	file::{fs::FileOps, vfs, vfs::mountpoint, File},
	format_content,
	process::{mem_space::copy::UserSlice, pid::Pid},
};
use core::{fmt, fmt::Formatter};
use utils::{errno::EResult, DisplayableStr};

/// The `mounts` node.
#[derive(Debug)]
pub struct Mounts(pub Pid);

impl FileOps for Mounts {
	fn read(&self, _file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		format_content!(off, buf, "{self}")
	}
}

impl fmt::Display for Mounts {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let mps = mountpoint::MOUNT_POINTS.lock();
		for (_, mp) in mps.iter() {
			let Ok(target) = vfs::Entry::get_path(&mp.root_entry) else {
				continue;
			};
			let fs_type = mp.fs.ops.get_name();
			let flags = "TODO"; // TODO
			writeln!(
				f,
				"{source} {target} {fs_type} {flags} 0 0",
				source = mp.source,
				target = target,
				fs_type = DisplayableStr(fs_type)
			)?;
		}
		Ok(())
	}
}
