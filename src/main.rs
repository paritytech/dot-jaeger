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
use env_logger::{Builder, Env};

mod api;
mod cli;
mod daemon;
mod graph;
mod http;
mod primitives;

fn main() -> Result<(), Error> {
	Builder::from_env(Env::default().default_filter_or("info")).init();

	cli::app()?;
	Ok(())
}

#[cfg(test)]
mod tests {
	// test data for child-parent relationships
	pub const TEST_DATA: &str = r#"
	{
	    "traceID": "6ga7nenJ21rhDy6Fwzjwz7KZQ5Jrii9",
        "spans": [
			{
 				"traceID": "6ga7nenJ21rhDy6Fwzjwz7KZQ5Jrii9",
				"spanID": "parent",
				"flags": null,
				"operationName": "testop",
				"references": [],
				"startTime": 1616995411000000,
				"duration": 150,
				"tags": [
					{
						"key": "otel.library.name",
						"type": "string",
						"value": "mick-jaeger"
					},
					{
						"key": "otel.library.version",
						"type": "string",
						"value": "0.1.4"
					},
					{
						"key": "candidate-stage",
						"type": "string",
						"value": "4"
					},
					{
						"key": "internal.span.format",
						"type": "string",
						"value": "proto"
					}
				],
				"logs": [],
				"processID": "p1",
				"warnings": null
			},
			{
				"traceID": "6ga7nenJ21rhDy6Fwzjwz7KZQ5Jrii9",
				"spanID": "child-0",
				"flags": null,
				"operationName": "testop",
				"references": [
					{
						"refType": "CHILD_OF",
						"traceID": "6ga7nenJ21rhDy6Fwzjwz7KZQ5Jrii9",
						"spanID": "parent"
					}
				],
				"startTime": 1616995411000000,
				"duration": 150,
				"tags": [
					{
						"key": "otel.library.name",
						"type": "string",
						"value": "mick-jaeger"
					},
					{
						"key": "otel.library.version",
						"type": "string",
						"value": "0.1.4"
					},
					{
						"key": "candidate-stage",
						"type": "string",
						"value": "4"
					},
					{
						"key": "internal.span.format",
						"type": "string",
						"value": "proto"
					}
				],
				"logs": [],
				"processID": "p1",
				"warnings": null
			},
			{
				"traceID": "6ga7nenJ21rhDy6Fwzjwz7KZQ5Jrii9",
				"spanID": "child-1",
				"flags": null,
				"operationName": "testop",
				"references": [
					{
						"refType": "CHILD_OF",
						"traceID": "6ga7nenJ21rhDy6Fwzjwz7KZQ5Jrii9",
						"spanID": "child-0"
					}
				],
				"startTime": 1616995411000000,
				"duration": 150,
				"tags": [
					{
						"key": "otel.library.name",
						"type": "string",
						"value": "mick-jaeger"
					},
					{
						"key": "otel.library.version",
						"type": "string",
						"value": "0.1.4"
					},
					{
						"key": "candidate-stage",
						"type": "string",
						"value": "4"
					},
					{
						"key": "internal.span.format",
						"type": "string",
						"value": "proto"
					}
				],
				"logs": [],
				"processID": "p1",
				"warnings": null
			},
			{
				"traceID": "6ga7nenJ21rhDy6Fwzjwz7KZQ5Jrii9",
				"spanID": "child-2",
				"flags": null,
				"operationName": "testop",
				"references": [
					{
						"refType": "CHILD_OF",
						"traceID": "6ga7nenJ21rhDy6Fwzjwz7KZQ5Jrii9",
						"spanID": "child-1"
					}
				],
				"startTime": 1616995411000000,
				"duration": 150,
				"tags": [
					{
						"key": "otel.library.name",
						"type": "string",
						"value": "mick-jaeger"
					},
					{
						"key": "otel.library.version",
						"type": "string",
						"value": "0.1.4"
					},
					{
						"key": "candidate-stage",
						"type": "string",
						"value": "4"
					},
					{
						"key": "internal.span.format",
						"type": "string",
						"value": "proto"
					}
				],
				"logs": [],
				"processID": "p1",
				"warnings": null
			}
		],
		"processes": {
      		"p1": {
        		"serviceName": "polkadot-insi-testing",
        		"tags": []
      		}
    	}
    }
    "#;
}
