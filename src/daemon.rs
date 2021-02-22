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

use crate::{api::JaegerApi, cli::App, primitives::Span};
use anyhow::Error;
use log::info;
use prometheus_exporter::prometheus::{register_gauge, Gauge};
use std::{
	collections::HashMap,
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
	pub fn new(port: usize, api: &'a JaegerApi, app: &'a App) -> Self {
		let metrics = Metrics::new();
		Self { port, api, app, metrics }
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
			if let Err(e) = self.collect_metrics() {
				log::error!("{}", e.to_string());
				running.store(false, Ordering::SeqCst);
				break;
			}
		}
		Ok(())
	}

	fn collect_metrics(&mut self) -> Result<(), Error> {
		let now = std::time::Instant::now();
		let traces = self.api.traces(self.app)?;
		println!("API Call took {:?} seconds", now.elapsed());
		println!("Total Traces: {}", traces.len());
		self.metrics.extend(traces.into_iter().map(|t| t.spans).flatten())?;

		info!("Updating metrics");
		// TODO: can do metric updating _in_ metrics as to not pollute this loop
		println!("Total Candidates: {}", self.metrics.candidates());
		self.metrics.parachain_total_candidates.set(self.metrics.candidates() as f64);
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
	no_candidate: Vec<Span>,
	parachain_total_candidates: Gauge,
}

impl Metrics {
	pub fn new() -> Self {
		let parachain_total_candidates =
			register_gauge!("parachain_total_candidates", "Total candidates registered on this node")
				.expect("can not create guage random_value_metric");

		Self { candidates: HashMap::new(), no_candidate: Vec::new(), parachain_total_candidates }
	}

	/// Inserts an item into the Candidate List.
	/// If this item does not exist, then the entry will be created from CandidateHash.
	pub fn insert(&mut self, span: Span) -> Result<(), Error> {
		if let Some((hash, stage)) = extract_from_span(&span)? {
			if let Some(v) = self.candidates.get_mut(&stage) {
				v.push(Candidate { hash, span });
			} else {
				let new = vec![Candidate { hash, span }];
				self.candidates.insert(stage, new);
			}
		} else {
			self.no_candidate.push(span)
		}
		Ok(())
	}

	/// Fallible equivalent to [`std::iter::Extend`] trait
	pub fn extend<T: IntoIterator<Item = Span>>(&mut self, iter: T) -> Result<(), Error> {
		for span in iter {
			self.insert(span)?;
		}
		Ok(())
	}

	/// Clear memory of candidates
	pub fn clear(&mut self) {
		self.candidates.clear();
		self.no_candidate.clear();
	}

	pub fn candidates(&self) -> usize {
		self.candidates.len()
	}
}

struct Candidate {
	hash: CandidateHash,
	span: Span,
}

/// Stage of execution this Candidate is in
#[derive(PartialEq, Debug, Hash, Eq)]
struct Stage(u8);

impl FromStr for Stage {
	type Err = Error;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let num: u8 = s.parse()?;
		Ok(Stage(num))
	}
}

/// Extract Hash and Stage from a span
fn extract_from_span(item: &Span) -> Result<Option<(CandidateHash, Stage)>, Error> {
	let hash_string = item.get_tag(HASH_IDENTIFIER);
	let stage = item.get_tag(STAGE_IDENTIFIER);

	let mut hash = [0u8; 32];
	hash_string.map(|h| hex::decode_to_slice(&h.value()[2..], &mut hash)).transpose()?;
	let stage = stage.map(|s| s.value().parse()).transpose()?;

	let hash: Option<[u8; 32]> = if [0u8; 32] == hash { None } else { Some(hash) };

	Ok(stage.and_then(|s| hash.map(|h| (h, s))))
}
