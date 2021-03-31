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

//! The HTTP Server that responds to Prometheus Requests

use anyhow::{anyhow, Context as _, Error};
use ascii::AsciiString;
use prometheus::{Encoder as _, TextEncoder};
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tiny_http::{Header, Request, Response, Server as TinyServer};

pub struct Server {
	handle: jod_thread::JoinHandle<()>,
	server: Arc<TinyServer>,
}

impl Server {
	pub fn start(addr: SocketAddr) -> Result<Self, Error> {
		let server = Arc::new(TinyServer::http(addr).map_err(|e| anyhow!(e.to_string()))?);
		let threaded_server = server.clone();
		log::info!("exporting metrics to http://{}/metrics", addr);

		let handle = jod_thread::spawn(move || {
			let mut instance = ServerInstance::new(&threaded_server);
			if let Err(e) = instance.request_handler() {
				log::error!("{}", e);
			}
		});

		Ok(Self { handle, server })
	}

	pub fn stop(self) {
		self.server.unblock();
		self.handle.join();
	}
}

struct ServerInstance<'a> {
	server: &'a TinyServer,
	time: Instant,
	requests_served: u32,
	last_buffer_length: usize,
}

impl<'a> ServerInstance<'a> {
	fn new(server: &'a TinyServer) -> Self {
		Self { server, time: Instant::now(), requests_served: 0, last_buffer_length: 0 }
	}

	fn request_handler(&mut self) -> Result<(), Error> {
		for request in self.server.incoming_requests() {
			match request.url() {
				"/metrics" => self.handle_metrics(request)?,
				_ => self.handle_redirect(request)?,
			};
			self.log_stats();
		}
		Ok(())
	}

	fn log_stats(&mut self) {
		self.requests_served += 1;
		let current_time = Instant::now();
		if current_time.duration_since(self.time).as_secs() > 60 {
			log::info!("[Server] Responded to {} requests", self.requests_served);
			log::info!("[Server] Last Buffer sent was {} bytes long", self.last_buffer_length);
		}
		self.time = Instant::now();
	}

	fn handle_metrics(&mut self, request: Request) -> Result<(), Error> {
		let encoder = TextEncoder::new();
		let metrics = prometheus::gather();
		let mut buffer = vec![];
		encoder.encode(&metrics, &mut buffer)?;
		self.last_buffer_length = buffer.len();
		let response = Response::from_data(buffer);
		request.respond(response).with_context(|| "Failed to respond to Prometheus request for metrics".to_string())?;
		Ok(())
	}

	fn handle_redirect(&mut self, request: Request) -> Result<(), Error> {
		let response = Response::from_string("the endpoint you probably want is `/metrics` ಠ_ಠ\n")
			.with_status_code(301)
			.with_header(Header {
				field: "Location".parse().expect("Can not parse location header. This should never fail"),
				value: AsciiString::from_ascii("/metrics")
					.expect("Could not parse header value. This should never fail."),
			});
		request.respond(response)?;
		Ok(())
	}
}
