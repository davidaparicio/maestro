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

//! Implementation of the `meminfo` file, allows to retrieve information about memory usage of the
//! system.

use crate::{
	file::{fs::FileOps, File},
	format_content, memory,
};
use utils::errno::EResult;

/// The `meminfo` file.
#[derive(Debug, Default)]
pub struct MemInfo;

impl FileOps for MemInfo {
	fn read(&self, _file: &File, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let mem_info = memory::stats::MEM_INFO.lock();
		format_content!(off, buf, "{}", *mem_info)
	}
}
