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

use anyhow::Error;
use serde::{de::Deserializer, Deserialize, Serialize};
use std::collections::HashMap;

/// RPC Primitives
#[derive(Serialize, Deserialize, Debug)]
pub struct RpcResponse<T> {
	data: Vec<T>,
	total: usize,
	limit: usize,
	offset: usize,
	errors: Option<serde_json::Value>,
}

impl<T> RpcResponse<T> {
	pub fn consume(self) -> Vec<T> {
		self.data
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TraceObject<'a> {
	#[serde(rename = "traceID")]
	trace_id: &'a str,
	#[serde(borrow, deserialize_with = "deserialize_vec_as_hashmap")]
	pub spans: HashMap<&'a str, Span<'a>>,
	#[serde(borrow)]
	processes: HashMap<&'a str, Process<'a>>,
	warnings: Option<Vec<&'a str>>,
}

fn deserialize_vec_as_hashmap<'de, D>(deserializer: D) -> Result<HashMap<&'de str, Span<'de>>, D::Error>
where
	D: Deserializer<'de>,
{
	let mut map = HashMap::new();
	for item in Vec::<Span<'de>>::deserialize(deserializer)? {
		map.insert(item.span_id, item);
	}
	Ok(map)
}

impl<'a> TraceObject<'a> {
	/// Finds a Span where `id` is listed as the child of another span.
	fn find_child(&self, id: &'a str) -> Option<&'a Span> {
		self.spans.values().find(|s| s.parent_span_id() == Some(id))
	}

	/// Gets a span that corresponds to the parent of the given id.
	fn get_parent(&self, id: &'a str) -> Option<&'a Span> {
		self.spans
			.get(id)
			.map(|s| {
				let parent_span = s.parent_span_id()?;
				self.spans.get(parent_span)
			})
			.flatten()
	}

	/// Recurse through a spans children, executing the predicate `fun` when a child is found.
	/// Recursing children can be very slow, since every child span must be searched for in the span list,
	/// since direct child-relationships are not included in the Jaeger Response.
	pub fn recurse_children<F>(&'a self, id: &'a str, mut fun: F) -> Result<(), Error>
	where
		F: FnMut(&'a Span<'a>) -> Result<bool, Error>,
	{
		if let Some(c) = self.find_child(id) {
			if !fun(c)? {
				self.recurse_children(c.span_id, fun)?;
			}
		}
		Ok(())
	}

	/// Recurse through a spans parents, applying the predicate `fun`.
	pub fn recurse_parents<F>(&'a self, id: &'a str, mut fun: F) -> Result<(), Error>
	where
		F: FnMut(&'a Span<'a>) -> Result<bool, Error>,
	{
		if let Some(c) = self.get_parent(id) {
			if !fun(c)? && c.parent_span_id().is_some() {
				self.recurse_parents(c.span_id, fun)?;
			}
		}
		Ok(())
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Span<'a> {
	#[serde(rename = "traceID")]
	pub trace_id: &'a str,
	#[serde(rename = "spanID")]
	pub span_id: &'a str,
	pub flags: Option<usize>,
	#[serde(rename = "operationName")]
	pub operation_name: &'a str,
	#[serde(borrow)]
	pub references: Vec<Reference<'a>>,
	#[serde(rename = "startTime")]
	pub start_time: usize,
	pub duration: f64,
	#[serde(borrow)]
	pub tags: Vec<Tag<'a>>,
	pub logs: Vec<serde_json::Value>, // FIXME: not sure what an actual 'log' looks like
	#[serde(rename = "processID")]
	pub process_id: &'a str,
	pub warnings: Option<Vec<&'a str>>,
}

impl<'a> Span<'a> {
	/// get a tag under `key`
	pub fn get_tag(&self, key: &str) -> Option<&'a Tag> {
		self.tags.iter().find(|t| t.key == key)
	}

	/// Get the ID to the parent of this span.
	pub fn parent_span_id(&self) -> Option<&'a str> {
		let child = self.references.iter().find(|r| r.ref_type == "CHILD_OF");
		child.map(|c| c.span_id)
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tag<'a> {
	key: &'a str,
	#[serde(rename = "type")]
	ty: &'a str,
	#[serde(borrow)]
	value: TagValue<'a>,
}

impl<'a> Tag<'a> {
	pub fn value(&self) -> String {
		self.value.to_string()
	}
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum TagValue<'a> {
	String(&'a str),
	Boolean(bool),
	Number(usize),
}

impl<'a> ToString for TagValue<'a> {
	fn to_string(&self) -> String {
		match self {
			TagValue::String(s) => s.to_string(),
			TagValue::Boolean(b) => b.to_string(),
			TagValue::Number(n) => n.to_string(),
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Process<'a> {
	#[serde(rename = "serviceName")]
	service_name: &'a str,
	#[serde(borrow)]
	tags: Vec<Tag<'a>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Reference<'a> {
	#[serde(rename = "refType")]
	ref_type: &'a str,
	#[serde(rename = "traceID")]
	trace_id: &'a str,
	#[serde(rename = "spanID")]
	span_id: &'a str,
}
