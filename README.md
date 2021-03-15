# Dot Jaeger

service for visualizing and collecting traces from Parachains.


start the external services (Prometheus + Grafana) with
```
docker-compose up
```



## Maintenence

#### Adding a new Stage

- Modify `Stage` enum and associated Into/From implementations to accomadate a new stage `stage.rs`
- Modify Prometheus Gauges to add new stage to Histograms `stage.rs`
