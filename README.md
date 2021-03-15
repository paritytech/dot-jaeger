# Dot Jaeger

service for visualizing and collecting traces from Parachains.

![Screenie](https://i.imgur.com/fdcqDjm.png)

## Guide
- Make sure you are on the Parity VPN if you plan to use that Jaeger UI Endpoint.

- Start the external services (Prometheus + Grafana) with
```
docker-compose up
```
This starts Prometheus on port 9090 and grafana on port 3000. The Grafana dashboard can be accessed from localhost:3000, with the default login being user: `admin` password: `admin`

- Start dot-jaeger in `daemon` mode with chosen arguments. The `help` command may be used for quick docs on the core app or any of the subcommands.

- Login to local grafana instance, and add `dot-jaeger` as a Prometheus source.
  - URL: `localhost:9090`
  - Access: Browser

- Import the Dashboard from the Repository named `Parachain Rococo Candidates-{{bunch of numbers}}`
  - dashboard can be manipulated from grafana

Data should start showing up. Grafana update interval can be modified in the top right


Recommended number of traces at once: 5-20. Asking for too many traces from the JaegerUI on Parity VPN both requests large amounts of data over the VPN and makes dot-jaeger slower as it has to potentially sort the parent-child relationship of each span, although this can be configured with `--recurse-children` and `recurse-parents` CLI options.

## Usage

``` sh
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

### Daemon

```sh
Usage: dot-jaeger daemon [--frequency <frequency>] [--port <port>] [--recurse-parents] [--recurse-children]

Daemonize Jaeger Trace collection to run at some interval

Options:
  --frequency       frequency to update jaeger metrics in milliseconds.
  --port            port to expose prometheus metrics at. Default 9186
  --recurse-parents fallback to recursing through parent traces if the current
                    span has one of a candidate hash or stage, but not the
                    other.
  --recurse-children
                    fallback to recursing through parent traces if the current
                    span has one of a candidate hash or stage but not the other.
                    Recursing children is slower than recursing parents.
  --help            display usage information


Usage: dot-jaeger daemon [--frequency <frequency>] [--port <port>] [--recurse-parents] [--recurse-children]

Daemonize Jaeger Trace collection to run at some interval

Options:
  --frequency       frequency to update jaeger metrics in milliseconds.
  --port            port to expose prometheus metrics at. Default 9186
  --recurse-parents fallback to recursing through parent traces if the current
                    span has one of a candidate hash or stage, but not the
                    other.
  --recurse-children
                    fallback to recursing through parent traces if the current
                    span has on of a candidate hash or stage but not the other.
                    Recursing children is slower than recursing parents.
  --help            display usage information
```


#### Example
./dot-jaeger --url "http://10.14.0.22:16686" --limit 10 --service polkadot-rococo-3-validator-5 daemon --recurse-children

## Maintenence

#### Adding a new Stage

- Modify `Stage` enum and associated Into/From implementations to accomadate a new stage `stage.rs`
- Modify Prometheus Gauges to add new stage to Histograms `stage.rs`
