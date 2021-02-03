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

mod api;
mod cli;

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
///

fn main() -> Result<(), Error> {
    cli::app();
    /*
        let body: String = ureq::get("http://localhost:16686/api/traces")
            .query("end", "1612290611587000")
            .query("start", "1612287011587000")
            .query("limit", "1")
            .query("service", "jaeger-query")
            .query("prettyPrint", "true")
            .call()?
            .into_string()?;

        println!("{}", body);
    */
    Ok(())
}
