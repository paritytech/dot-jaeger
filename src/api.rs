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

use crate::cli::{App, Trace};
use anyhow::Error;
use std::fmt;

/// Endpoints:
///
/// `/api/traces`
/// Params:
///     limit: specify how many to return
///     service: Where did the trace originate
///     prettyPrint: Make JSON nice
/// `/search` <-- have not gotten this to work
/// `/api/traces/{TraceId}`
///     return spans for this TraceId
/// `/api/services`
/// 	- returns services reporting to the jaeger agent
pub const TRACES: &str = "/api/traces";

/// Returns list of services on this Jaeger agent
pub const SERVICES: &str = "/api/services";

pub enum Endpoint {
    Traces,
    Services,
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Endpoint::Traces => write!(f, "{}", TRACES),
            Endpoint::Services => write!(f, "{}", SERVICES),
        }
    }
}

pub struct JaegerApi<'a> {
    /// URL Where Jaeger Agent is running.
    /// Should be full URL including Port and protocol.
    /// # Example
    /// http://localhost:16686
    url: &'a str,
}

impl<'a> JaegerApi<'a> {
    pub fn new(url: &'a str) -> Self {
        Self { url }
    }

    /// Get many traces belonging to one service from this Jaeger Agent.
    pub fn traces(&self, app: &App) -> Result<String, Error> {
        let req = ureq::get(&endpoint(self.url, Endpoint::Traces));
        let req = build_parameters(req, app);
        let response = req.call()?.into_string()?;
        Ok(response)
    }

    /// Get a single trace from the Jaeger Agent
    pub fn trace(&self, app: &App, trace: &Trace) -> Result<String, Error> {
        // /api/traces/{trace_id}
        let req = ureq::get(&format!(
            "{}/{}",
            &endpoint(self.url, Endpoint::Traces),
            trace.id.to_string()
        ));
        let req = build_parameters(req, app);
        let response = req.call()?.into_string()?;
        Ok(response)
    }

    /// Query the services that reporting to this Jaeger Agent
    pub fn services(&self, app: &App) -> Result<String, Error> {
        let req = ureq::get(&endpoint(&self.url, Endpoint::Services));
        let req = build_parameters(req, app);
        let response = req.call()?.into_string()?;
        Ok(response)
    }
}

fn build_parameters(req: ureq::Request, app: &App) -> ureq::Request {
    ParamBuilder::new(&app.service)
        .limit(app.limit)
        .pretty_print(app.pretty_print)
        .build(req)
}

fn endpoint(url: &str, endpoint: Endpoint) -> String {
    format!("{}/{}", url, endpoint)
}

// TODO: Params to Implement
// Lookback : How far back in time to look
// minDuration
// maxDuration
// operation
// start <- Unix timestamp in microseconds (presumably for internal Jaeger Use)
// end <- Unix timestamp in microseconds (presumably for internal Jaeger Use)
pub struct ParamBuilder<'a> {
    limit: Option<usize>,
    pretty_print: bool,
    service: &'a str,
}

impl<'a> ParamBuilder<'a> {
    pub fn new(service: &'a str) -> Self {
        Self {
            pretty_print: false,
            limit: None,
            service,
        }
    }

    pub fn limit(mut self, limit: Option<usize>) -> Self {
        self.limit = limit;
        self
    }

    pub fn pretty_print(mut self, pretty_print: bool) -> Self {
        self.pretty_print = pretty_print;
        self
    }

    pub fn build(self, req: ureq::Request) -> ureq::Request {
        let mut req = req
            .query("service", &self.service)
            .query("prettyPrint", &self.pretty_print.to_string());

        if let Some(limit) = self.limit {
            req = req.query("limit", &limit.to_string());
        }

        req
    }
}
