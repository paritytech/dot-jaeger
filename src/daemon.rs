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
use prometheus_exporter::prometheus::register_gauge;
use prometheus_exporter::{self, prometheus::register_counter};
use rand::Rng;
use std::{
	collections::HashMap,
	iter::Extend,
	net::SocketAddr,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
};

pub const HASH_IDENTIFIER: &str = "candidate-hash";

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
		let duration = std::time::Duration::from_millis(5000);

		// Create metric
		let parachain_total_candidates =
			register_gauge!("parachain_total_candidates", "Total candidates registered on this node")
				.expect("can not create guage random_value_metric");

		let running = Arc::new(AtomicBool::new(true));
		let r = running.clone();
		ctrlc::set_handler(move || r.store(false, Ordering::SeqCst)).expect("Could not set the Ctrl-C handler.");

		loop {
			let _guard = exporter.wait_duration(duration);

			let traces = self.api.traces(self.app)?;
			self.metrics.extend(traces.into_iter().map(|t| t.spans).flatten());

			info!("Updating metrics");
			// TODO: can do metric updating _in_ metrics as to not pollute this loop
			parachain_total_candidates.set(self.metrics.candidates() as f64);

			if !running.load(Ordering::SeqCst) {
				break;
			}
		}
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
	candidates: HashMap<CandidateHash, Vec<Span>>,
	no_candidate: Vec<Span>,
}

impl Metrics {
	pub fn new() -> Self {
		Self { candidates: HashMap::new(), no_candidate: Vec::new() }
	}

	/// Inserts an item into the Candidate List.
	/// If this item does not exist, then the entry will be created from CandidateHash.
	pub fn insert(&mut self, hash: &CandidateHash, item: Span) {
		if let Some(vec) = self.candidates.get_mut(hash) {
			vec.push(item);
		} else {
			let mut new = Vec::new();
			new.push(item);
			self.candidates.insert(*hash, new);
		}
	}

	pub fn candidates(&self) -> usize {
		self.candidates.len()
	}
}

impl Extend<Span> for Metrics {
	fn extend<T: IntoIterator<Item = Span>>(&mut self, iter: T) {
		for span in iter {
			if let Some(tag) = span.get_tag(HASH_IDENTIFIER) {
				let mut hash: CandidateHash = [0u8; 32];
				hex::decode_to_slice(tag.value(), &mut hash).unwrap();
				self.insert(&hash, span);
			} else {
				self.no_candidate.push(span);
			}
		}
	}
}
