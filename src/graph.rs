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
use anyhow::Error;
use daggy::{Dag, NodeIndex, Walker};
use std::collections::HashMap;

const EDGE_WEIGHT: u32 = 1;
type DirectedGraph<'a> = Dag<Span<'a>, u32, u32>;

#[derive(Debug)]
pub struct Graph<'a> {
	trace: &'a TraceObject<'a>,
	graph: DirectedGraph<'a>,
	/// Dictionary of the nodes present in the graph
	index_lookup: HashMap<&'a str, NodeIndex<u32>>,
	span_lookup: HashMap<NodeIndex, &'a str>,
}

impl<'a> Graph<'a> {
	pub fn new(trace: &'a TraceObject<'a>) -> Result<Self, Error> {
		let mut graph = Dag::new();
		let mut index_lookup = HashMap::new();
		let mut span_lookup = HashMap::new();

		for span in trace.spans.values() {
			let index = graph.add_node(span.clone());
			index_lookup.insert(span.span_id, index);
			span_lookup.insert(index, span.span_id);
		}

		for id in span_lookup.values() {
			if let Some(parent) = trace.get_parent(id) {
				let parent_node = index_lookup.get(&parent.span_id).unwrap();
				let index = index_lookup.get(id).unwrap();
				graph.add_edge(*parent_node, *index, EDGE_WEIGHT)?;
			}
		}

		Ok(Self { trace, graph, index_lookup, span_lookup })
	}

	pub fn children(&'a self, id: &'a str) -> Option<impl Iterator<Item = &'a Span<'a>>> {
		let id = self.index_lookup.get(id)?;
		Some(self.graph.children(*id).iter(&self.graph).filter_map(move |(_, n)| self.get_span_by_index(&n)))
	}

	pub fn parents(&'a self, id: &'a str) -> Option<impl Iterator<Item = &'a Span<'a>>> {
		let id = self.index_lookup.get(id)?;
		Some(self.graph.parents(*id).iter(&self.graph).filter_map(move |(_, n)| self.get_span_by_index(&n)))
	}

	fn get_span_by_index(&'a self, index: &NodeIndex<u32>) -> Option<&'a Span<'a>> {
		let span_id = self.span_lookup.get(index)?;
		self.trace.spans.get(span_id)
	}
}
