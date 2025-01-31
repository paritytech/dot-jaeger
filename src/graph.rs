// Copyright 2021 Parity Technologies (UK) Ltd.
// This file is part of dot-jaeger.

// dot-jaeger is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// dot-jaeger is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with dot-jaeger.  If not, see <http://www.gnu.org/licenses/>.

use crate::primitives::{Span, TraceObject};
use anyhow::{Context, Error};
use daggy::{Dag, NodeIndex, Walker};
use petgraph::visit::Dfs;
use std::collections::HashMap;

const EDGE_WEIGHT: u32 = 1;
type DirectedGraph<'a> = Dag<Span<'a>, u32, u32>;

#[derive(Debug)]
pub struct Graph<'a> {
	trace: &'a TraceObject<'a>,
	graph: DirectedGraph<'a>,
	/// Dictionary of the nodes present in the graph
	index_lookup: HashMap<&'a str, NodeIndex<u32>>,
}

impl<'a> Graph<'a> {
	/// Instantiate a new graph object for span traversal.
	pub fn new(trace: &'a TraceObject<'a>) -> Result<Self, Error> {
		let mut graph = Dag::new();
		let mut index_lookup = HashMap::new();

		for span in trace.spans.values() {
			let index = graph.add_node(span.clone());
			index_lookup.insert(span.span_id, index);
		}

		for id in trace.spans.values().map(|s| s.span_id) {
			if let Some(parent) = trace.get_parent(id) {
				let parent_node = index_lookup.get(&parent.span_id).unwrap();
				let index = index_lookup.get(id).unwrap();
				graph.add_edge(*parent_node, *index, EDGE_WEIGHT)?;
			}
		}

		Ok(Self { trace, graph, index_lookup })
	}

	/// Do a depth-first search for a span that meets the requirements of the predicate `fun`.
	pub fn search(&'a self, id: &'a str) -> Result<impl Iterator<Item = &'a Span<'a>>, Error> {
		let span_node = self.index_lookup.get(id).context(format!("Span {} not found in index", id))?;

		let depth_first = Dfs::new(&self.graph, *span_node);
		Ok(depth_first.iter(&self.graph).map(move |n| &self.graph.raw_nodes()[n.index()].weight))
	}

	/// Recursively walk through the parents of a span.
	pub fn parents(&'a self, id: &'a str) -> Result<impl Iterator<Item = &'a Span<'a>>, Error> {
		let id = self.index_lookup.get(id).context(format!("Parent span {} not found in index", id))?;
		let iter = self.graph.recursive_walk(*id, |rgraph, n| rgraph.parents(n).iter(&rgraph).nth(0));
		Ok(iter.iter(&self.graph).map(move |(_, n)| &self.graph.raw_nodes()[n.index()].weight))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tests::*;

	#[test]
	fn should_iter_parents() -> Result<(), Error> {
		let traces: TraceObject = serde_json::from_str(TEST_DATA)?;
		let graph = Graph::new(&traces)?;

		let mut iterator = graph.parents("child-2").unwrap();
		assert_eq!(Some("child-1"), iterator.next().map(|s| s.span_id));
		assert_eq!(Some("child-0"), iterator.next().map(|s| s.span_id));
		assert_eq!(Some("parent"), iterator.next().map(|s| s.span_id));
		assert_eq!(None, iterator.next().map(|s| s.span_id));
		Ok(())
	}

	#[test]
	fn should_iter_children() -> Result<(), Error> {
		let traces: TraceObject = serde_json::from_str(TEST_DATA)?;
		let graph = Graph::new(&traces)?;

		let mut iterator = graph.search("parent")?;
		assert_eq!(Some("parent"), iterator.next().map(|s| s.span_id));
		assert_eq!(Some("child-0"), iterator.next().map(|s| s.span_id));
		assert_eq!(Some("child-1"), iterator.next().map(|s| s.span_id));
		assert_eq!(Some("child-2"), iterator.next().map(|s| s.span_id));

		Ok(())
	}
}
