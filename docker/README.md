# Dockerization for the Knowledge Graph Project

This directory contains the Docker setup for the `knowledge-graph` project. The containerization is organized into a multi-service application managed by Docker Compose, using a single, efficient multi-stage `Dockerfile`.

## Directory Structure

The `docker/` directory contains the core Docker assets:

-   `Dockerfile`: A multi-stage Dockerfile that builds all the necessary service images (`gkg`, `webserver`, `indexer`, `desktop`).
-   `push.sh`: A utility script to build and push all service images to a Docker registry.
-   `README.md`: This file.

## Services

The `docker-compose.yml` file at the root of the repository defines the following services:

-   `gkg`: A service for the `gkg` command-line interface, used for batch indexing of local workspaces.
-   `webserver`: The main back-end API server for handling queries.
-   `indexer`: The back-end service for handling API-driven, on-demand indexing of single repositories.
-   `desktop`: The local development server that provides the graph visualization UI.

## Building and Running the Application

To build and run the entire stack, you can use Docker Compose from the root of the repository.

### Prerequisites

-   Docker and Docker Compose must be installed on your system.
-   A `jwt.secret` file must be present in the root directory for the `webserver` and `indexer` services to start. You can create one for local development by running:
    ```bash
    echo "dummy-secret-for-development" > jwt.secret
    ```

### Build the Images

To build all the Docker images for the services, run the following command from the root of the repository:

```bash
docker compose build
```

### Run the Services

To start all services in detached mode, run:

```bash
docker compose up -d
```

You can view the logs for all services using:

```bash
docker compose logs -f
```

Or for a specific service:

```bash
docker compose logs -f <service_name>
```

(e.g., `docker compose logs -f webserver`)

### Stop the Services

To stop all running services, run:

```bash
docker compose down
```

## Pushing to a Docker Registry

The `docker/push.sh` script is provided to build and push all images to a container registry. Before running, ensure you are logged in (`docker login`) and have updated the script with your registry username.

To execute the script from the project root, run:

```bash
sh docker/push.sh
```
