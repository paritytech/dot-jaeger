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

use log::info;
use prometheus_exporter::prometheus::register_gauge;
use prometheus_exporter::{self, prometheus::register_counter};
use rand::Rng;
use std::net::SocketAddr;

pub struct PromDaemon {
	port: usize,
}

impl PromDaemon {
	pub fn new(port: usize) -> Self {
		Self { port }
	}

	pub fn start(&self) {
		let addr_raw = format!("0.0.0.0:{}", self.port);
		let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

		// start the exporter and update metrics every five seconds
		let exporter = prometheus_exporter::start(addr).expect("can not start exporter");
		let duration = std::time::Duration::from_millis(5000);

		// Create metric
		let random = register_gauge!("run_and_repeat_random", "will set a random value")
			.expect("can not create guage random_value_metric");

		let mut rng = rand::thread_rng();

		loop {
			let _guard = exporter.wait_duration(duration);

			info!("Updating metrics");

			let new_value = rng.gen();
			info!("New random value: {}", new_value);
			random.set(new_value);
		}

		let body = ureq::get(&format!("http://{}/metrics", addr_raw))
			.call()
			.expect("can not get metrics from exporter")
			.into_string()
			.expect("Can not get body");
		info!("Exporter metrics:\n {}", body);
	}
}
