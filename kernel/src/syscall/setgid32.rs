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

//! The `setgid32` syscall sets the GID of the process's owner.

use crate::{file::perm::Gid, process::Process, syscall::Args};
use utils::errno::{EResult, Errno};

pub fn setgid32(Args(gid): Args<Gid>) -> EResult<usize> {
	let proc_mutex = Process::current();
	let mut proc = proc_mutex.lock();

	proc.access_profile.set_gid(gid)?;
	Ok(0)
}
