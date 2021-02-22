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

use serde::{Deserialize, Serialize};
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
	#[serde(borrow)]
	pub spans: Vec<Span<'a>>,
	#[serde(borrow)]
	processes: HashMap<&'a str, Process<'a>>,
	warnings: Option<Vec<&'a str>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Span<'a> {
	#[serde(rename = "traceID")]
	trace_id: &'a str,
	#[serde(rename = "spanID")]
	span_id: &'a str,
	flags: Option<usize>,
	#[serde(rename = "operationName")]
	operation_name: &'a str,
	#[serde(borrow)]
	references: Vec<Reference<'a>>,
	#[serde(rename = "startTime")]
	start_time: usize,
	duration: usize,
	#[serde(borrow)]
	tags: Vec<Tag<'a>>,
	logs: Vec<serde_json::Value>, // FIXME: not sure what an actual 'log' looks like
	#[serde(rename = "processID")]
	process_id: &'a str,
	warnings: Option<Vec<&'a str>>,
}

impl<'a> Span<'a> {
	pub fn get_tag(&self, key: &str) -> Option<&'a Tag> {
		self.tags.iter().find(|t| t.key == key)
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
