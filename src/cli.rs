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

use crate::api::JaegerApi;

#[derive(FromArgs, PartialEq, Debug)]
/// Jaeger Trace CLI App
pub struct App {
    #[argh(option)]
    /// name a specific node that reports to the Jaeger Agent from which te query traces.
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
struct AllTraces {
    #[argh(option, default = "String::from(\"\")")] // default is no filter
    /// filter these Traces with Regex
    filter: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "services")]
/// List of services reporting to the Jaeger Agent
struct Services {
    #[argh(option, default = "String::from(\"\")")] // default is no filter
    /// regex to apply and filter results
    filter: String,
}

pub fn app() -> Result<(), Error> {
    let app: App = argh::from_env();

    match &app.action {
        TraceAction::AllTraces(_) => traces(&app)?,
        TraceAction::Trace(trace_opts) => trace(&app, &trace_opts)?,
        TraceAction::Services(_) => services(&app)?,
    }
    Ok(())
}

/// Return All Traces.
fn traces(app: &App) -> Result<(), Error> {
    let api = JaegerApi::new(&app.url);
    let data = api.traces(app)?;
    if app.pretty_print {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        println!("{}", serde_json::to_string(&data)?);
    }
    Ok(())
}

/// Get a span by its Hex String ID
fn trace(app: &App, trace: &Trace) -> Result<(), Error> {
    let api = JaegerApi::new(&app.url);
    let data = api.trace(app, trace)?;
    if app.pretty_print {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        println!("{}", serde_json::to_string(&data)?);
    }

    Ok(())
}

fn services(app: &App) -> Result<(), Error> {
    let api = JaegerApi::new(&app.url);
    let data = api.services(app)?;
    println!("{}", data);
    Ok(())
}
