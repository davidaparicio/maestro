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

//! Filesystem node cache, allowing to handle hard links pointing to the same node.

use crate::{
	file::{
		fs::{FileOps, Filesystem, NodeOps},
		vfs::mountpoint::MountPoint,
		FileLocation, FileType, INode,
	},
	memory::buddy::PageState,
	sync::mutex::Mutex,
};
use core::{
	borrow::Borrow,
	hash::{Hash, Hasher},
};
use utils::{
	boxed::Box,
	collections::{hashmap::HashSet, vec::Vec},
	errno::{AllocResult, EResult},
	ptr::arc::Arc,
};

/// A filesystem node, cached by the VFS.
#[derive(Debug)]
pub struct Node {
	/// Node ID
	pub inode: INode,
	/// Mountpoint of the filesystem on which the node is located
	pub mp: Arc<MountPoint>,
	/// Handle for node operations
	pub node_ops: Box<dyn NodeOps>,
	/// Handle for open file operations
	pub file_ops: Box<dyn FileOps>,
	// TODO need a sparse array, inside of a rwlock
	/// Mapped pages
	pub pages: Mutex<Vec<&'static PageState>>,
}

impl Node {
	/// Returns a reference to the underlying filesystem.
	pub fn get_filesystem(&self) -> &dyn Filesystem {
		&*self.mp.fs
	}

	/// Releases the node, removing it from the disk if this is the last reference to it.
	pub fn release(this: Arc<Self>) -> EResult<()> {
		// Lock to avoid race condition later
		let mut used_nodes = USED_NODES.lock();
		// current instance + the one in `USED_NODE` = `2`
		if Arc::strong_count(&this) > 2 {
			return Ok(());
		}
		used_nodes.remove(&this.location);
		let Some(node) = Arc::into_inner(this) else {
			return Ok(());
		};
		Self::try_remove(&node.location, &*node.node_ops)
	}

	/// Removes the node from the disk if it is orphan.
	///
	/// Arguments:
	/// - `loc` is the location of the node
	/// - `ops` is the handle to perform operations on the node
	fn try_remove(loc: &FileLocation, ops: &dyn NodeOps) -> EResult<()> {
		// If there is no hard link left to the node, remove it
		let stat = ops.get_stat(loc)?;
		let dir = stat.get_type() == Some(FileType::Directory);
		// If the file is a directory, the threshold is `1` because of the `.` entry
		let remove = (dir && stat.nlink <= 1) || stat.nlink == 0;
		if remove {
			ops.remove_node(loc)?;
		}
		Ok(())
	}
}

/// An entry in the nodes cache.
///
/// The [`Hash`] and [`PartialEq`] traits are forwarded to the entry's location.
#[derive(Debug)]
struct NodeEntry(Arc<Node>);

impl Borrow<FileLocation> for NodeEntry {
	fn borrow(&self) -> &FileLocation {
		&self.0.location
	}
}

impl Eq for NodeEntry {}

impl PartialEq for NodeEntry {
	fn eq(&self, other: &Self) -> bool {
		self.0.location.eq(&other.0.location)
	}
}

impl Hash for NodeEntry {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.location.hash(state)
	}
}

/// The list of nodes current in use.
static USED_NODES: Mutex<HashSet<NodeEntry>> = Mutex::new(HashSet::new());

/// Looks in the nodes cache for the node with the given location. If not in cache, the node is
/// created and inserted.
pub(super) fn lookup(location: FileLocation) -> Option<Arc<Node>> {
	USED_NODES.lock().get(&location).map(|e| e.0.clone())
}

/// Inserts a new node in cache.
pub(super) fn insert(node: Arc<Node>) -> AllocResult<()> {
	USED_NODES.lock().insert(NodeEntry(node))?;
	Ok(())
}

/// The function removes the node from:
/// - the cache if no reference to it is taken
/// - the filesystem if it is orphan
///
/// Arguments:
/// - `loc` is the location of the node
/// - `ops` is the handle to perform operations on the node
pub(super) fn try_remove(loc: &FileLocation, ops: &dyn NodeOps) -> EResult<()> {
	let mut used_nodes = USED_NODES.lock();
	// Remove from cache
	if let Some(NodeEntry(node)) = used_nodes.get(loc) {
		// If the node is referenced elsewhere, stop
		if Arc::strong_count(node) > 1 {
			return Ok(());
		}
		used_nodes.remove(loc);
	}
	// Remove the node
	Node::try_remove(loc, ops)
}
