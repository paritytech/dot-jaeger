use anyhow::Error;

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
    let body: String = ureq::get("http://localhost:16686/api/traces")
        .query("end", "1612290611587000")
        .query("start", "1612287011587000")
        .query("limit", "1")
        .query("service", "jaeger-query")
        .query("prettyPrint", "true")
        .call()?
        .into_string()?;
    println!("{}", body);
    Ok(())
}
