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
	#[serde(deserialize_with = "deserialize_vec_as_hashmap")]
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
	/// Gets a span that corresponds to the parent of the given id.
	pub fn get_parent(&self, id: &'a str) -> Option<&'a Span> {
		self.spans
			.get(id)
			.map(|s| {
				let parent_span = s.parent_span_id()?;
				self.spans.get(parent_span)
			})
			.flatten()
	}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
	#[serde(borrow)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Reference<'a> {
	#[serde(rename = "refType")]
	ref_type: &'a str,
	#[serde(rename = "traceID")]
	trace_id: &'a str,
	#[serde(rename = "spanID")]
	span_id: &'a str,
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tests::*;
	use anyhow::Error;

	#[test]
	fn should_find_parents() -> Result<(), Error> {
		let traces: TraceObject = serde_json::from_str(TEST_DATA)?;
		assert_eq!(traces.get_parent("child-0").unwrap().span_id, "parent");
		assert_eq!(traces.get_parent("child-1").unwrap().span_id, "child-0");
		Ok(())
	}
}
