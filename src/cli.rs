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
use argh::FromArgs;

use crate::{api::JaegerApi, daemon::PrometheusDaemon, primitives::TraceObject};

#[derive(FromArgs, PartialEq, Debug)]
/// Jaeger Trace CLI App
pub struct App {
	#[argh(option)]
	/// name a specific node that reports to the Jaeger Agent from which to query traces.
	pub service: Option<String>,
	#[argh(option, default = "String::from(\"http://localhost:16686\")")]
	/// URL where Jaeger Service runs.
	pub url: String,
	#[argh(option)]
	/// maximum number of traces to return.
	pub limit: Option<usize>,
	#[argh(switch)]
	/// pretty print result
	pub pretty_print: bool,
	#[argh(option)]
	/// specify how far back in time to look for traces. In format: `1h`, `1d`
	pub lookback: Option<String>,
	#[argh(subcommand)]
	/// what action to perform on Jaeger Service.
	action: TraceAction,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum TraceAction {
	AllTraces(AllTraces),
	Trace(Trace),
	Services(Services),
	Daemon(Daemon),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "trace")]
/// Use when observing only one trace
pub struct Trace {
	#[argh(option)]
	/// the hex string ID of the trace to get. Example: --id 3c58a09870e2dced
	pub id: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "traces")]
/// Use when observing many traces
pub struct AllTraces {
	#[argh(option)] // default is no filter
	/// filter these Traces with Regex
	filter: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "services")]
/// List of services reporting to the Jaeger Agent
pub struct Services {
	#[argh(option)] // default is no filter
	/// regex to apply and filter results
	filter: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "daemon")]
/// Daemonize Jaeger Trace collection to run at some interval
pub struct Daemon {
	#[argh(option)]
	/// frequency to update jaeger metrics in milliseconds.
	pub frequency: Option<usize>,
	#[argh(option, default = "default_port()")]
	/// port to expose prometheus metrics at. Default 9186
	pub port: usize,
	/// fallback to recursing through parent traces if the current span has one of a candidate hash or stage, but not the other.
	#[argh(switch)]
	pub recurse_parents: bool,
	#[argh(switch)]
	/// fallback to recursing through parent traces if the current span has one of a candidate hash or stage but not the other.
	/// Recursing children is slower than recursing parents.
	pub recurse_children: bool,
}

const fn default_port() -> usize {
	9186
}

pub fn app() -> Result<(), Error> {
	let app: App = argh::from_env();

	match &app.action {
		TraceAction::AllTraces(all_traces) => traces(&app, &all_traces)?,
		TraceAction::Trace(trace_opts) => trace(&app, &trace_opts)?,
		TraceAction::Services(serv) => services(&app, &serv)?,
		TraceAction::Daemon(daemon) => daemonize(&app, daemon)?,
	}
	Ok(())
}

/// Return All Traces.
fn traces(app: &App, _: &AllTraces) -> Result<(), Error> {
	let api = JaegerApi::new(&app.url);
	let data = api.traces(app)?;
	let json = api.into_json::<TraceObject>(&data)?;
	if app.pretty_print {
		println!("{}", serde_json::to_string_pretty(&json)?);
	} else {
		println!("{}", serde_json::to_string(&json)?);
	}
	Ok(())
}

/// Get a span by its Hex String ID
fn trace(app: &App, trace: &Trace) -> Result<(), Error> {
	let api = JaegerApi::new(&app.url);
	let data = api.trace(app, &trace.id)?;
	let json = api.into_json::<TraceObject>(&data)?;
	if app.pretty_print {
		println!("{}", serde_json::to_string_pretty(&json)?);
	} else {
		println!("{}", serde_json::to_string(&json)?);
	}

	Ok(())
}

/// Get a list of services reporting to the Jaeger Agent and print them out.
fn services(app: &App, _: &Services) -> Result<(), Error> {
	let api = JaegerApi::new(&app.url);
	let data = api.services(app)?;
	for item in data.iter() {
		println!("{}", item);
	}
	Ok(())
}

/// Daemonize collecting Jaeger Metrics every few seconds, reporting everything to Prometheus.
fn daemonize(app: &App, daemon: &Daemon) -> Result<(), Error> {
	let api = JaegerApi::new(&app.url);
	println!("Launching Jaeger Collector daemon!");
	let mut daemon = PrometheusDaemon::new(daemon, &api, app)?;
	daemon.start()?;
	Ok(())
}
