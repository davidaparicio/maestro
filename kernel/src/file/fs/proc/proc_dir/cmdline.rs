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

//! The `cmdline` node allows to retrieve the list of command line arguments of
//! the process.

use super::read_memory;
use crate::{
	file::{fs::FileOps, File},
	format_content,
	process::{pid::Pid, Process},
};
use core::fmt;
use utils::{errno, errno::EResult};

/// The cmdline node of the proc.
#[derive(Clone, Debug)]
pub struct Cmdline(pub Pid);

impl FileOps for Cmdline {
	fn read(&self, _file: &File, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let proc = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let mem_space = proc.mem_space.as_ref().unwrap().lock();
		let disp = fmt::from_fn(|f| {
			read_memory(
				f,
				&mem_space,
				mem_space.exe_info.argv_begin,
				mem_space.exe_info.argv_end,
			)
		});
		format_content!(off, buf, "{disp}")
	}
}
