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
use anyhow::{Error, bail};
use prometheus_exporter::prometheus::{labels, register_gauge, register_histogram, histogram_opts, linear_buckets, Histogram, Gauge};
use std::{
	collections::HashMap,
	net::SocketAddr,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
	convert::TryFrom,
	time::Duration,
};
use rand::Rng;

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
		self.metrics.update(traces.iter().map(|t| t.spans.iter()).flatten())?;
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
	// no_candidate: Vec<Span<'a>>,
	parachain_total_candidates: Gauge,
	parachain_histogram: Histogram,
}

impl Metrics {
	pub fn new() -> Result<Self, Error> {
		let parachain_total_candidates =
			register_gauge!("parachain_total_candidates", "Total candidates registered on this node")
			.expect("can not create guage parachain_total_candidates metric");

		let histogram_opts = histogram_opts!(
			"parachain_histogram",
			"track stage of candidates",
			linear_buckets(0f64, 1f64, 10)?,
			labels!{
				"Stage 0".to_string()  => "Stage 0".to_string(),
				"Stage 1".to_string()  => "Stage 1".to_string(),
				"Stage 2".to_string()  => "Stage 2".to_string(),
				"Stage 3".to_string()  => "Stage 3".to_string(),
				"Stage 4".to_string()  => "Stage 4".to_string(),
				"Stage 5".to_string()  => "Stage 5".to_string(),
				"Stage 6".to_string()  => "Stage 6".to_string(),
				"Stage 7".to_string()  => "Stage 7".to_string(),
				"Stage 8".to_string()  => "Stage 8".to_string(),
				"Stage 9".to_string()  => "Stage 9".to_string(),
				"Stage 10".to_string() => "Stage 10".to_string()
			}
		);
		let parachain_histogram = register_histogram!(histogram_opts)?;
		Ok(Self { candidates: HashMap::new(), parachain_total_candidates, parachain_histogram })
	}

	fn update<'a>(&mut self, traces: impl Iterator<Item = &'a Span<'a>>) -> Result<(), Error> {
		for span in traces {
			self.insert(span)?;
		}
		for (k, v) in self.candidates.iter() {
			for _ in 0..v.len() { self.parachain_histogram.observe(*k as f64); }
		}
		self.parachain_total_candidates.set(self.candidates.len() as f64);

		Ok(())
	}

	/// Inserts an item into the Candidate List.
	/// If this item does not exist, then the entry will be created from CandidateHash.
	pub fn insert<'a>(&mut self, span: &'a Span<'a>) -> Result<(), Error> {
		// TODO: Placeholder randomized stage
		let stage = rand::thread_rng().gen_range(0..10);
		if extract_from_span(span)?.is_some() {
			if let Some(v) = self.candidates.get_mut(&stage) {
				v.push(Candidate::try_from(span)?);
			} else {
				let new = vec![Candidate::try_from(span)?];
				self.candidates.insert(stage, new);
			}
		} else {
			// self.no_candidate.push(span)
		}
		Ok(())
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

struct Candidate {
	hash: CandidateHash,
	operation: String,
}

impl<'a> TryFrom<&'a Span<'a>> for Candidate {
	type Error = Error;
	fn try_from(span: &'a Span<'a>) -> Result<Candidate, Error> {
		let hash_string =  span.get_tag(HASH_IDENTIFIER);

		let mut hash = [0u8; 32];
		hash_string.map(|h| hex::decode_to_slice(&h.value()[2..], &mut hash)).transpose()?;
		if [0u8; 32] == hash { bail!("no hash for candidate") } else { hash };

		Ok(Candidate {
			hash,
			operation: span.operation_name.to_string(),
		})
	}
}

/// Stage of execution this Candidate is in
type Stage = u8;
/*
impl FromStr for Stage {
	type Err = Error;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let num: u8 = s.parse()?;
		Ok(num)
	}
}
*/
/// Extract Hash and Stage from a span
fn extract_from_span(item: &Span) -> Result<Option<Stage>, Error> {
	let stage = item.get_tag(STAGE_IDENTIFIER);
	// let stage = stage.map(|s| s.value().parse()).transpose()?;
	// TODO: PLACEHOLDER STAGE
	let stage  = Some(0);
	Ok(stage)
}
