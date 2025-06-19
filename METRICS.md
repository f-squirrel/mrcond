# Prometheus Metrics

The MongoDB-RabbitMQ Connector provides Prometheus metrics to monitor the health and performance of running connector tasks.

## Metrics Endpoint

The metrics are exposed at the `/metrics` endpoint, which is available on the same port as the health check endpoint (default: 3000).

**URL:** `http://localhost:3000/metrics`

## Available Metrics

### Core Metrics

| Metric Name | Type | Description | Labels |
|-------------|------|-------------|---------|
| `mrcon_running_servers_total` | Gauge | Total number of running connector servers | None |
| `mrcon_collection_servers` | Gauge | Number of connector servers per collection | `collection`, `database` |
| `mrcon_tasks_started_total` | Counter | Total number of tasks started since server startup | None |
| `mrcon_task_restarts_total` | Counter | Total number of task restarts per collection | `collection`, `database`, `reason` |
| `mrcon_task_failures_total` | Counter | Total number of task failures per collection | `collection`, `database`, `error_type` |

### Metric Details

#### `mrcon_running_servers_total`
- **Type:** Gauge
- **Description:** Shows the current number of active connector tasks running in the JoinSet
- **Use Case:** Monitor overall system load and detect if tasks are failing to start

#### `mrcon_collection_servers`
- **Type:** Gauge
- **Description:** Shows how many connector instances are running for each collection
- **Labels:**
  - `collection`: Name of the MongoDB collection
  - `database`: Name of the MongoDB database
- **Use Case:** Monitor per-collection connector status

#### `mrcon_tasks_started_total`
- **Type:** Counter
- **Description:** Cumulative count of all tasks started since the server began
- **Use Case:** Track task lifecycle and restart frequency

#### `mrcon_task_restarts_total`
- **Type:** Counter
- **Description:** Count of task restarts broken down by reason
- **Labels:**
  - `collection`: Name of the MongoDB collection
  - `database`: Name of the MongoDB database
  - `reason`: Reason for restart (`mongo_connection_failed`, `rabbitmq_connection_failed`, `unhandled_error`)
- **Use Case:** Identify connection issues and system stability

#### `mrcon_task_failures_total`
- **Type:** Counter
- **Description:** Count of task failures broken down by error type
- **Labels:**
  - `collection`: Name of the MongoDB collection
  - `database`: Name of the MongoDB database
  - `error_type`: Type of error (`mongo_error`, `rabbitmq_error`, `unknown_error`, `task_panic`)
- **Use Case:** Troubleshoot specific types of failures

## Example Queries

### Prometheus Queries

```promql
# Current number of running servers
mrcon_running_servers_total

# Number of servers per collection
mrcon_collection_servers

# Rate of task failures over the last 5 minutes
rate(mrcon_task_failures_total[5m])

# Rate of task restarts over the last 5 minutes
rate(mrcon_task_restarts_total[5m])

# Total tasks started
mrcon_tasks_started_total
```

### Sample Metrics Output

```
# HELP mrcon_running_servers_total Total number of running connector servers
# TYPE mrcon_running_servers_total gauge
mrcon_running_servers_total 2

# HELP mrcon_collection_servers Number of connector servers per collection
# TYPE mrcon_collection_servers gauge
mrcon_collection_servers{collection="users",database="myapp"} 1
mrcon_collection_servers{collection="orders",database="myapp"} 1

# HELP mrcon_tasks_started_total Total number of tasks started since server startup
# TYPE mrcon_tasks_started_total counter
mrcon_tasks_started_total 2

# HELP mrcon_task_restarts_total Total number of task restarts per collection
# TYPE mrcon_task_restarts_total counter
mrcon_task_restarts_total{collection="users",database="myapp",reason="mongo_connection_failed"} 0

# HELP mrcon_task_failures_total Total number of task failures per collection
# TYPE mrcon_task_failures_total counter
mrcon_task_failures_total{collection="users",database="myapp",error_type="mongo_error"} 0
```

## Monitoring Setup

### Prometheus Configuration

Add the following to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'mrcon'
    static_configs:
      - targets: ['localhost:3000']
```

### Grafana Dashboard

Key metrics to monitor in Grafana:

1. **System Overview Panel:**
   - `mrcon_running_servers_total` - Current active servers
   - `rate(mrcon_tasks_started_total[5m])` - Task start rate

2. **Error Monitoring Panel:**
   - `rate(mrcon_task_failures_total[5m])` - Failure rate
   - `rate(mrcon_task_restarts_total[5m])` - Restart rate

3. **Per-Collection Panel:**
   - `mrcon_collection_servers` - Servers per collection

### Alerting Rules

Example Prometheus alerting rules:

```yaml
groups:
  - name: mrcon
    rules:
      - alert: MrconNoRunningServers
        expr: mrcon_running_servers_total == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "No MongoDB-RabbitMQ connector servers running"
          description: "All connector servers have stopped running"

      - alert: MrconHighFailureRate
        expr: rate(mrcon_task_failures_total[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High failure rate for MongoDB-RabbitMQ connector"
          description: "Task failure rate is {{ $value }} failures per second"

      - alert: MrconHighRestartRate
        expr: rate(mrcon_task_restarts_total[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High restart rate for MongoDB-RabbitMQ connector"
          description: "Task restart rate is {{ $value }} restarts per second"
```

## Testing

You can test the metrics endpoint using curl:

```bash
# Check if metrics endpoint is available
curl http://localhost:3000/metrics

# Check health endpoint
curl http://localhost:3000/health
```

## Integration with Docker Compose

The metrics endpoint is exposed on port 3000 in the docker-compose setup. You can access it at:

```bash
# From host machine
curl http://localhost:3000/metrics

# From within docker network
curl http://connector:3000/metrics
```
