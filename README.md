# Dot Jaeger

service for visualizing and collecting traces from Parachains.


start the external services (Prometheus + Grafana) with
```
docker-compose up
```



## Usage

```
Usage: dot-jaeger [--service <service>] [--url <url>] [--limit <limit>] [--pretty-print] [--lookback <lookback>] <command> [<args>]

Jaeger Trace CLI App

Options:
  --service         name a specific node that reports to the Jaeger Agent from
                    which to query traces.
  --url             URL where Jaeger Service runs.
  --limit           maximum number of traces to return.
  --pretty-print    pretty print result
  --lookback        specify how far back in time to look for traces. In format:
                    `1h`, `1d`
  --help            display usage information

Commands:
  traces            Use when observing many traces
  trace             Use when observing only one trace
  services          List of services reporting to the Jaeger Agent
  daemon            Daemonize Jaeger Trace collection to run at some interval
```

#### Example
./dot-jaeger --url "http://10.14.0.22:16686" --limit 10 --service polkadot-rococo-3-validator-5 daemon --recurse-children

## Maintenence

#### Adding a new Stage

- Modify `Stage` enum and associated Into/From implementations to accomadate a new stage `stage.rs`
- Modify Prometheus Gauges to add new stage to Histograms `stage.rs`
