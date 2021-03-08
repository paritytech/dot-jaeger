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
use anyhow::{bail, Error};
use itertools::Itertools;
use prometheus_exporter::prometheus::{register_gauge, register_histogram, Gauge, Histogram};
use std::{
	collections::HashMap,
	convert::TryFrom,
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
/// Default for Histogram Buckets.
/// Buckets ranging from 250-20,000 milliseconds in steps of 250 milliseconds
/// modifying this constant will modify all histogram buckets.
pub const HISTOGRAM_BUCKETS: &[f64; 80] = &[
	250.0, 500.0, 750.0, 1000.0, 1250.0, 1500.0, 1750.0, 2000.0, 2250.0, 2500.0, 2750.0, 3000.0, 3250.0, 3500.0,
	3750.0, 4000.0, 4250.0, 4500.0, 4750.0, 5000.0, 5250.0, 5500.0, 5750.0, 6000.0, 6250.0, 6500.0, 6750.0, 7000.0,
	7250.0, 7500.0, 7750.0, 8000.0, 8250.0, 8500.0, 8750.0, 9000.0, 9250.0, 9500.0, 9750.0, 10_000.0, 10_250.0,
	10_500.0, 10_750.0, 11_000.0, 11_250.0, 11_500.0, 11_750.0, 12_000.0, 12_250.0, 12_500.0, 12_750.0, 13_000.0,
	13_250.0, 13_500.0, 13_750.0, 14_000.0, 15_250.0, 15_500.0, 15_750.0, 16_000.0, 16_250.0, 16_500.0, 16_750.0,
	17_000.0, 17_250.0, 17_500.0, 17_750.0, 18_000.0, 18_250.0, 18_500.0, 18_750.0, 19_000.0, 19_250.0, 19_500.0,
	19_750.0, 20_000.0, 20_250.0, 20_500.0, 20_750.0, 21_000.0,
];

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
		println!("Deserialization took {:?}", now.elapsed());
		println!("Total Traces: {}", traces.len());
		let now = std::time::Instant::now();
		self.metrics.update(traces.iter().map(|t| t.spans.iter()).flatten().collect())?;
		println!("Updating took {:?}", now.elapsed());
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
	parachain_stage_histograms: [Histogram; 8],
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

		let parachain_stage_histograms = [
			register_histogram!(
				"stage_0_duration",
				"Distributions of the time it takes to transition between stages",
				HISTOGRAM_BUCKETS.to_vec()
			)?,
			register_histogram!(
				"stage_1_duration",
				"Distributions of the time it takes to transition between stages",
				HISTOGRAM_BUCKETS.to_vec()
			)?,
			register_histogram!(
				"stage_2_duration",
				"Distributions of the time it takes to transition between stages",
				HISTOGRAM_BUCKETS.to_vec()
			)?,
			register_histogram!(
				"stage_3_duration",
				"Distributions of the time it takes to transition between stages",
				HISTOGRAM_BUCKETS.to_vec()
			)?,
			register_histogram!(
				"stage_4_duration",
				"Distributions of the time it takes to transition between stages",
				HISTOGRAM_BUCKETS.to_vec()
			)?,
			register_histogram!(
				"stage_5_duration",
				"Distributions of the time it takes to transition between stages",
				HISTOGRAM_BUCKETS.to_vec()
			)?,
			register_histogram!(
				"stage_6_duration",
				"Distributions of the time it takes to transition between stages",
				HISTOGRAM_BUCKETS.to_vec()
			)?,
			register_histogram!(
				"stage_7_duration",
				"Distributions of the time it takes to transition between stages",
				HISTOGRAM_BUCKETS.to_vec()
			)?,
		];

		Ok(Self {
			candidates: HashMap::new(),
			parachain_total_candidates,
			parachain_stage_gauges,
			parachain_stage_histograms,
		})
	}

	fn update<'a>(&mut self, traces: Vec<&'a Span<'a>>) -> Result<(), Error> {
		let mut no_candidates = Vec::new();
		let mut no_stage = Vec::new();
		for span in traces.iter() {
			if span.get_tag(HASH_IDENTIFIER).is_none() {
				no_candidates.push(*span);
			} else if span.get_tag(STAGE_IDENTIFIER).is_none() {
				no_stage.push(*span);
			} else {
				self.insert(span)?;
			}
		}

		let now = std::time::Instant::now();
		let map = SpanMap::new(traces.as_slice());
		self.try_resolve_missing_candidates(&map, &mut no_candidates)?;
		self.try_resolve_missing_stage(&map, &mut no_stage)?;
		println!("Resolving missing candidates took {:?}", now.elapsed());

		// Distribution of Candidate Stage deltas
		for stage in self.candidates.keys() {
			if let Some(c) = self.candidates.get(&stage) {
				for candidate in c.iter() {
					// Jaeger stores durations in microseconds. We divide by 1000 to get milliseconds.
					self.parachain_stage_histograms[*stage as usize].observe(candidate.duration / 1000f64)
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

		let with_stage =
			no_candidates.iter().filter(|c| c.get_tag(STAGE_IDENTIFIER).is_some()).collect::<Vec<_>>().len();

		println!(
			"Candidates with a hash but without a stage: {:?}",
			self.candidates.get(&Stage::NoStage).map(|c| c.len())
		);
		println!("Spans without a candidate-hash but with an associated stage: {}", with_stage);

		Ok(())
	}

	/// Inserts an item into the Candidate List.
	pub fn insert<'a>(&mut self, span: &'a Span<'a>) -> Result<(), Error> {
		let stage = extract_stage_from_span(span)?.unwrap_or(Stage::NoStage);
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

	// Fallback in case some candidates are missing a candidate-hash but have a stage.
	// checks if the parent span has a candidate-hash attached.
	fn try_resolve_missing_candidates<'a>(
		&mut self,
		map: &SpanMap<'a>,
		no_candidates: &mut Vec<&'a Span<'a>>,
	) -> Result<(), Error> {
		let mut to_remove = Vec::new();
		for missing in no_candidates.iter() {
			if let Some(id) = missing.get_child_span_id() {
				if let Some(parent) = map.get(id) {
					if parent.get_tag(HASH_IDENTIFIER).is_some() {
						let stage = extract_stage_from_span(missing)?.unwrap_or(Stage::NoStage);
						let hash = extract_hash_from_span(&parent)?.expect("Hash must exist because of tag check; qed");
						let candidate = Candidate::from_other_hash(missing, hash);
						if let Some(v) = self.candidates.get_mut(&stage) {
							v.push(candidate)
						} else {
							self.candidates.insert(stage, vec![candidate]);
						}
						to_remove.push(missing.span_id);
					}
				}
			}
		}
		no_candidates.retain(|x| to_remove.iter().any(|&r| r == x.span_id));
		Ok(())
	}

	// Fallback for spans where some spans are missing a stage but have a candidate-hash
	fn try_resolve_missing_stage<'a>(
		&mut self,
		map: &SpanMap<'a>,
		no_stage: &mut Vec<&'a Span<'a>>,
	) -> Result<(), Error> {
		let mut to_remove = Vec::new();
		for missing in no_stage.iter() {
			if let Some(id) = missing.get_child_span_id() {
				if let Some(parent) = map.get(id) {
					if parent.get_tag(STAGE_IDENTIFIER).is_some() {
						let stage = extract_stage_from_span(parent)?.unwrap_or(Stage::NoStage);
						println!("Found Stage!: {}", stage);
						if let Some(candidate) = Option::<Candidate>::try_from(*missing)? {
							if let Some(v) = self.candidates.get_mut(&stage) {
								v.push(candidate);
							} else {
								self.candidates.insert(stage, vec![candidate]);
							}
							to_remove.push(missing.span_id);
						}
					}
				}
			}
		}
		no_stage.retain(|x| to_remove.iter().any(|&r| r == x.span_id));
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
	duration: f64,
}

impl Candidate {
	fn from_other_hash<'a>(span: &'a Span, hash: CandidateHash) -> Self {
		Candidate {
			hash,
			operation: span.operation_name.to_string(),
			start_time: span.start_time,
			duration: span.duration,
		}
	}
}

impl<'a> TryFrom<&'a Span<'a>> for Option<Candidate> {
	type Error = Error;
	fn try_from(span: &'a Span<'a>) -> Result<Option<Candidate>, Error> {
		let hash = extract_hash_from_span(span)?;

		Ok(hash.map(|h| Candidate {
			hash: h,
			operation: span.operation_name.to_string(),
			start_time: span.start_time,
			duration: span.duration,
		}))
	}
}

/// Extract Hash and Stage from a span
fn extract_stage_from_span(item: &Span) -> Result<Option<Stage>, Error> {
	let stage = item.get_tag(STAGE_IDENTIFIER);
	let stage = stage.map(|s| s.value().parse()).transpose()?;
	Ok(stage)
}

fn extract_hash_from_span(span: &Span) -> Result<Option<CandidateHash>, Error> {
	let hash_string = span.get_tag(HASH_IDENTIFIER);
	let mut hash = [0u8; 32];
	hash_string.map(|h| hex::decode_to_slice(&h.value()[2..], &mut hash)).transpose()?;
	if [0u8; 32] == hash {
		Ok(None)
	} else {
		Ok(Some(hash))
	}
}

/// Temporary structure to optimize fetching of specific spans
/// HashMap of SpanId -> Span
struct SpanMap<'a> {
	spans: HashMap<&'a str, &'a Span<'a>>,
}

impl<'a> SpanMap<'a> {
	fn new(spans: &'a [&'a Span<'a>]) -> Self {
		let mut map = HashMap::new();
		for span in spans {
			map.insert(span.span_id, *span);
		}
		Self { spans: map }
	}

	fn get(&self, id: &'a str) -> Option<&'a Span<'a>> {
		self.spans.get(id).map(|s| *s)
	}
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
