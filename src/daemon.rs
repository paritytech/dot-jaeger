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

//! Prometheus Daemon that exports metrics to some port.

use crate::{
	api::JaegerApi,
	cli::App,
	primitives::{Span, TraceObject},
};
use anyhow::{anyhow, bail, Error};
use itertools::Itertools;
use prometheus_exporter::prometheus::{
	histogram_opts, labels, linear_buckets, register_gauge, register_histogram_vec, Gauge, HistogramVec,
};
use std::{
	collections::HashMap,
	convert::{TryFrom, TryInto},
	iter::Iterator,
	net::SocketAddr,
	str::FromStr,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	time::Duration,
};

pub const HASH_IDENTIFIER: &str = "candidate-hash";
pub const STAGE_IDENTIFIER: &str = "candidate-stage";

pub type CandidateHash = [u8; 32];

pub struct PrometheusDaemon<'a> {
	port: usize,
	api: &'a JaegerApi<'a>,
	app: &'a App,
	metrics: Metrics,
}

impl<'a> PrometheusDaemon<'a> {
	pub fn new(port: usize, api: &'a JaegerApi, app: &'a App) -> Result<Self, Error> {
		let metrics = Metrics::new()?;
		Ok(Self { port, api, app, metrics })
	}

	pub fn start(&mut self) -> Result<(), Error> {
		let addr_raw = format!("0.0.0.0:{}", self.port);
		let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

		// start the exporter and update metrics every five seconds
		let exporter = prometheus_exporter::start(addr).expect("can not start exporter");

		let running = Arc::new(AtomicBool::new(true));
		let r = running.clone();
		ctrlc::set_handler(move || r.store(false, Ordering::SeqCst)).expect("Could not set the Ctrl-C handler.");

		while running.load(Ordering::SeqCst) {
			let _guard = exporter.wait_duration(Duration::from_millis(1000));
			self.metrics.clear();
			let now = std::time::Instant::now();
			let json = self.api.traces(self.app)?;
			println!("API Call took {:?} seconds", now.elapsed());
			if let Err(e) = self.collect_metrics(&json) {
				log::error!("{}", e.to_string());
				running.store(false, Ordering::SeqCst);
				break;
			}
		}
		Ok(())
	}

	fn collect_metrics(&mut self, json: &str) -> Result<(), Error> {
		let now = std::time::Instant::now();
		let traces = self.api.into_json::<TraceObject>(json)?;
		println!("Deserialization took {:?} seconds", now.elapsed());
		println!("Total Traces: {}", traces.len());
		self.metrics.update(traces.iter().map(|t| t.spans.iter()).flatten().collect())?;
		Ok(())
	}
}

// TODO:
// - Need to group candidates by their parent span ID
// - Organize Candidates by the 'stage' tag (not yet implemented in substrate)
// 		- once stage tag is implemented, we can track how many/which candidates reach the end of the cycle
//
/// Objects that tracks metrics per-candidate.
/// Keeps spans without a candidate in a separate list, for potential reference.
struct Metrics {
	candidates: HashMap<Stage, Vec<Candidate>>,
	parachain_total_candidates: Gauge,
	// the `zero` stage signifies a candidate that has no stage associated
	parachain_stage_gauges: [Gauge; 8],
	stage_transitions_delta: HistogramVec,
}

impl Metrics {
	pub fn new() -> Result<Self, Error> {
		let parachain_total_candidates =
			register_gauge!("parachain_total_candidates", "Total candidates registered on this node")
				.expect("can not create gauge parachain_total_candidates metric");
		let parachain_stage_gauges = [
			register_gauge!("stage_0_candidates", "Total Candidates without an associated stage")
				.expect("can not create gauge stage_0_candidates metric"),
			register_gauge!("stage_1_candidates", "Total Candidates on Stage 1")
				.expect("can not create gauge stage_1_candidates metric"),
			register_gauge!("stage_2_candidates", "Total Candidates on Stage 2")
				.expect("can not create gauge stage_2_candidates metric"),
			register_gauge!("stage_3_candidates", "Total Candidates on Stage 3")
				.expect("can not create gauge stage_3_candidates metric"),
			register_gauge!("stage_4_candidates", "Total Candidates on Stage 4")
				.expect("can not create gauge stage_4_candidates metric"),
			register_gauge!("stage_5_candidates", "Total Candidates on Stage 5")
				.expect("can not create gauge stage_5_candidates metric"),
			register_gauge!("stage_6_candidates", "Total Candidates on Stage 6")
				.expect("can not create gauge stage_6_candidates metric"),
			register_gauge!("stage_7_candidates", "Total Candidates on Stage 7")
				.expect("can not create gauge stage_7_candidates metric"),
		];

		let stage_transitions_delta = register_histogram_vec!(
			"stage_transitions_delta",
			"Distributions of the time it takes to transition between stages",
			&["stage_1", "stage_2", "stage_3", "stage_4", "stage_5", "stage_6", "stage_7"]
		)?;

		Ok(Self {
			candidates: HashMap::new(),
			parachain_total_candidates,
			parachain_stage_gauges,
			stage_transitions_delta,
		})
	}

	fn update<'a>(&mut self, traces: Vec<&'a Span<'a>>) -> Result<(), Error> {
		let mut no_candidates = Vec::new();
		for span in traces.iter() {
			if span.get_tag(HASH_IDENTIFIER).is_none() {
				no_candidates.push(*span);
			} else {
				self.insert(span)?;
			}
		}

		// Distribution of Candidate Stage deltas
		for stage in self.candidates.keys() {
			if let Some(c) = self.candidates.get(&stage) {
				for candidate in c.iter() {
					self.stage_transitions_delta
						.local()
						.with_label_values(&[&format!("stage_{}", stage)])
						// TODO FIXME: as loses 64 bit precision. Use something like the `conv` crate
						.observe(candidate.duration as f64);
				}
			}
		}

		// # Candidates in Each Stage
		for (i, gauge) in self.parachain_stage_gauges.iter().enumerate() {
			let count = self
				.candidates
				.get(&Stage::try_from(i)?)
				.map(|c| c.iter().unique_by(|c| c.hash).collect::<Vec<_>>().len());
			if let Some(c) = count {
				gauge.set(c as f64);
			}
		}
		// Total Number of Candidates
		let count: usize = self.candidates.values().flatten().unique_by(|c| c.hash).collect::<Vec<_>>().len();
		self.parachain_total_candidates.set(count as f64);

		Ok(())
	}

	/// Inserts an item into the Candidate List.
	pub fn insert<'a>(&mut self, span: &'a Span<'a>) -> Result<(), Error> {
		let stage = extract_from_span(span)?.unwrap_or(Stage::NoStage);
		if let Some(v) = self.candidates.get_mut(&stage) {
			let candidate: Option<Candidate> = TryFrom::try_from(span)?;
			if let Some(c) = candidate {
				v.push(c);
			}
		} else {
			let candidate: Option<Candidate> = Option::try_from(span)?;
			if let Some(c) = candidate {
				self.candidates.insert(stage, vec![c]);
			}
		}
		Ok(())
	}

	pub fn try_resolve_missing_candidates<'a>(&mut self, spans: Vec<&'a Span>, no_candidates: &[&'a Span<'a>]) {
		for missing in no_candidates.iter() {
			if let Some(f) = spans.iter().find(|s| s.span_id == missing.span_id) {
				println!("Found Candidate with tags {:?} that is a parent", f.tags);
			}
		}
	}

	/// Fallible equivalent to [`std::iter::Extend`] trait
	pub fn extend<'a, T: IntoIterator<Item = &'a Span<'a>>>(&mut self, iter: T) -> Result<(), Error> {
		for span in iter {
			self.insert(span)?;
		}
		Ok(())
	}

	/// Clear memory of candidates
	pub fn clear(&mut self) {
		self.candidates.clear();
	}
}

#[derive(Debug, PartialEq)]
struct Candidate {
	hash: CandidateHash,
	operation: String,
	start_time: usize,
	duration: usize,
}

impl<'a> TryFrom<&'a Span<'a>> for Option<Candidate> {
	type Error = Error;
	fn try_from(span: &'a Span<'a>) -> Result<Option<Candidate>, Error> {
		let hash_string = span.get_tag(HASH_IDENTIFIER);

		let mut hash = [0u8; 32];
		hash_string.map(|h| hex::decode_to_slice(&h.value()[2..], &mut hash)).transpose()?;
		if [0u8; 32] == hash {
			Ok(None)
		} else {
			Ok(Some(Candidate {
				hash,
				operation: span.operation_name.to_string(),
				start_time: span.start_time,
				duration: span.duration,
			}))
		}
	}
}
/*
fn find_parent<'a>(id: &'a str, spans: impl Iterator<Item = &'a Span<'a>>) -> Option<&'a Span<'a>> {
	spans.find(|s| s.span_id == id)
}
*/
/// Extract Hash and Stage from a span
fn extract_from_span(item: &Span) -> Result<Option<Stage>, Error> {
	let stage = item.get_tag(STAGE_IDENTIFIER);
	let stage = stage.map(|s| s.value().parse()).transpose()?;
	Ok(stage)
}

// TODO: Consider just importing polkadot 'jaeger' crate
/// A helper to annotate the stage with a numerical value
/// to ease the life of the tooling team creating viable
/// statistical metrics for which stage of the inclusion
/// pipeline drops a significant amount of candidates,
/// statistically speaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum Stage {
	NoStage = 0,
	CandidateSelection = 1,
	CandidateBacking = 2,
	StatementDistribution = 3,
	PoVDistribution = 4,
	AvailabilityDistribution = 5,
	AvailabilityRecovery = 6,
	BitfieldDistribution = 7,
	// Expand as needed, numbers should be ascending according to the stage
	// through the inclusion pipeline, or according to the descriptions
	// in [the path of a para chain block]
	// (https://polkadot.network/the-path-of-a-parachain-block/)
	// see [issue](https://github.com/paritytech/polkadot/issues/2389)
}

impl FromStr for Stage {
	type Err = Error;
	fn from_str(s: &str) -> Result<Self, Error> {
		match s.parse()? {
			0 => Ok(Stage::NoStage),
			1 => Ok(Stage::CandidateSelection),
			2 => Ok(Stage::CandidateBacking),
			3 => Ok(Stage::StatementDistribution),
			4 => Ok(Stage::PoVDistribution),
			5 => Ok(Stage::AvailabilityDistribution),
			6 => Ok(Stage::AvailabilityRecovery),
			7 => Ok(Stage::BitfieldDistribution),
			_ => bail!(format!("stage {} does not exist", s)),
		}
	}
}

impl TryFrom<usize> for Stage {
	type Error = Error;
	fn try_from(num: usize) -> Result<Stage, Error> {
		match num {
			0 => Ok(Stage::NoStage),
			1 => Ok(Stage::CandidateSelection),
			2 => Ok(Stage::CandidateBacking),
			3 => Ok(Stage::StatementDistribution),
			4 => Ok(Stage::PoVDistribution),
			5 => Ok(Stage::AvailabilityDistribution),
			6 => Ok(Stage::AvailabilityRecovery),
			7 => Ok(Stage::BitfieldDistribution),
			_ => bail!(format!("stage {} does not exist", num)),
		}
	}
}

impl std::fmt::Display for Stage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", (*self as usize))
	}
}
