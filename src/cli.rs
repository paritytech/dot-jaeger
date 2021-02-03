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
    #[argh(option, default = "String::from(\"jaeger-query\")")]
    /// service description
    pub service: String,
    #[argh(option, default = "String::from(\"http://localhost:16686\")")]
    /// url description
    pub url: String,
    #[argh(option)]
    /// which trace to start at
    pub start: Option<usize>,
    #[argh(option)]
    /// which trace to end at
    pub end: Option<usize>,
    #[argh(option)]
    /// maximum number of traces to return
    pub limit: Option<usize>,
    #[argh(switch)]
    /// pretty print result
    pub pretty_print: bool,
    #[argh(subcommand)]
    /// action description
    action: TraceAction,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum TraceAction {
    AllTraces(AllTraces),
    Trace(Trace),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "trace")]
/// Use when observing only one trace
struct Trace {
    #[argh(option)]
    #[argh(description = "id desc")]
    id: String,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "traces")]
/// Use when observing bulk traces
struct AllTraces {
    #[argh(option)]
    /// filter these Traces with Regex
    filter: String,
}

pub fn app() -> Result<(), Error> {
    let app: App = argh::from_env();

    match &app.action {
        TraceAction::AllTraces(AllTraces { filter, .. }) => traces(&app)?,
        TraceAction::Trace(Trace { id, .. }) => unimplemented!(),
    }
    Ok(())
}

/// Return All Traces.
fn traces(app: &App) -> Result<(), Error> {
    let api = JaegerApi::new(&app.url, &app.service);
    println!("{}", api.traces(app)?);
    Ok(())
}

/// Get a trace by it's hex string
fn trace() {
    todo!();
}
