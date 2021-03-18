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
use std::{net::SocketAddr, sync::Arc};
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
			if let Err(e) = Self::request_handler(&threaded_server) {
				log::error!("{}", e);
			}
		});

		Ok(Self { handle, server })
	}

	fn request_handler(server: &TinyServer) -> Result<(), Error> {
		for request in server.incoming_requests() {
			match request.url() {
				"/metrics" => Self::handle_metrics(request)?,
				_ => Self::handle_redirect(request)?,
			};
		}
		Ok(())
	}

	fn handle_metrics(request: Request) -> Result<(), Error> {
		let encoder = TextEncoder::new();
		let metrics = prometheus::gather();
		let mut buffer = vec![];

		encoder.encode(&metrics, &mut buffer)?;

		let response = Response::from_data(buffer);
		request.respond(response).with_context(|| "Failed to respond to Prometheus request for metrics".to_string())?;
		Ok(())
	}

	fn handle_redirect(request: Request) -> Result<(), Error> {
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

	pub fn stop(self) {
		self.server.unblock();
		self.handle.join();
	}
}
