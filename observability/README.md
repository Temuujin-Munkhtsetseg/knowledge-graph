# GitLab Knowldege Graph Observability Stack

This directory contains the local observability stack for the GKG (GitLab Knowledge Graph) project. It mirrors the production setup with Prometheus for metrics collection, Grafana for visualization, and Grafana Mimir for long-term metrics storage.

## Quick Start

### Start the observability stack
```bash
mise run observability:up
```

### Stop the observability stack
```bash
mise run observability:down
```

## Components

### Prometheus
- **URL**: http://localhost:9090
- **Purpose**: Metrics collection and short-term storage
- Scrapes metrics from the GKG HTTP server every 15 seconds
- Forwards metrics to Mimir for long-term storage via remote write

### Grafana
- **URL**: http://localhost:3001
- **Credentials**: admin / admin
- **Purpose**: Visualization and dashboards
- Pre-configured with:
  - Prometheus datasource (default)
  - Mimir datasource for long-term metrics
  - GKG Overview Dashboard

### Grafana Mimir
- **URL**: http://localhost:9009
- **Purpose**: Long-term metrics storage and querying
- Configured in single-process mode for local development
- Stores metrics in the local filesystem

## Configuration

### Prometheus Configuration
Edit `prometheus/prometheus.yml` to:
- Add new scrape targets
- Adjust scrape intervals
- Configure alerting rules

**Note**: By default, Prometheus expects the GKG HTTP server to be running on `host.docker.internal:8080` (the default port). If you use the `--bind` flag with a different port, update the target in `prometheus.yml` to match.

### Grafana Dashboards
- Dashboards are located in `grafana/provisioning/dashboards/json/`
- New dashboards can be added to this directory and will be auto-loaded
- Edit existing dashboards through the Grafana UI and save

### Mimir Configuration
Edit `mimir/mimir.yaml` to adjust:
- Storage backend settings
- Ingestion rate limits
- Retention policies

## Data Persistence

All metrics data is stored in Docker volumes:
- `prometheus-data` - Prometheus TSDB
- `mimir-data` - Mimir blocks and metadata
- `grafana-data` - Grafana dashboards and settings

To reset all data:
```bash
mise run observability:clean
```

## Troubleshooting

### Metrics not showing up in Grafana
1. Check if the GKG HTTP server is running and accessible
2. Verify Prometheus can scrape the metrics endpoint:
   - Go to http://localhost:9090/targets
   - Check if `gkg-http-server-deployed` target is UP
3. Check container logs:
   ```bash
   docker-compose logs -f prometheus
   docker-compose logs -f grafana
   docker-compose logs -f mimir
   ```

### Can't access services
- Ensure no other services are using ports 3001, 9009, or 9090
- Check if Docker is running
- Verify containers are running: `docker-compose ps`

### Connection refused from Prometheus to GKG server
- On macOS/Windows, Docker uses `host.docker.internal` to access host services
- On Linux, you may need to use the host's IP address or configure the docker-compose to use host network mode

## Running the HTTP Server

The deployed HTTP server now defaults to TCP binding on `127.0.0.1:8080`:

```bash
# Run with default port (8080)
cargo run --bin http-server-deployed -- --secret-path /path/to/secret

# Run on a different port
cargo run --bin http-server-deployed -- --bind 127.0.0.1:9090 --secret-path /path/to/secret
```

