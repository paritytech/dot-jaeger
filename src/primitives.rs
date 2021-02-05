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
pub struct TraceObject {
    #[serde(rename = "traceID")]
    trace_id: String,
    spans: Vec<Span>,
    processes: HashMap<String, Process>,
    warnings: Option<serde_json::Value>, // FIXME: Don't know what actual value of 'warnings' looks like
}

#[derive(Serialize, Deserialize, Debug)]
struct Span {
    #[serde(rename = "traceID")]
    trace_id: String,
    #[serde(rename = "spanID")]
    span_id: String,
    flags: Option<usize>,
    #[serde(rename = "operationName")]
    operation_name: String,
    references: Vec<Reference>,
    #[serde(rename = "startTime")]
    start_time: usize,
    duration: usize,
    tags: Vec<Tag>,
    logs: Vec<serde_json::Value>, // FIXME: not sure what an actual 'log' looks like
    #[serde(rename = "processID")]
    process_id: String,
    warnings: Option<serde_json::Value>, // FIXME: not sure what the actual value for 'warnings' looks like
}

#[derive(Serialize, Deserialize, Debug)]
struct Tag {
    key: String,
    #[serde(rename = "type")]
    ty: String,
    value: TagValue,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum TagValue {
    String(String),
    Boolean(bool),
    Number(usize),
}

#[derive(Serialize, Deserialize, Debug)]
struct Process {
    #[serde(rename = "serviceName")]
    service_name: String,
    tags: Vec<Tag>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Reference {
	#[serde(rename = "refType")]
	ref_type: String,
	#[serde(rename = "traceID")]
	trace_id: String,
	#[serde(rename = "spanID")]
	span_id: String
}
