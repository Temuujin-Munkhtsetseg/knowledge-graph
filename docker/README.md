# Dockerization for the Knowledge Graph Project

This directory contains the Docker setup for the `knowledge-graph` project. The containerization is organized into a multi-service application managed by Docker Compose.

## Directory Structure

The `docker/` directory is structured to provide a clean and separate build environment for each application:

-   `docker/gkg/`: Contains the `Dockerfile` for the `gkg` binary.
-   `docker/http-server-deployed/`: Contains the `Dockerfile` for the `http-server-deployed` binary, which is the web-facing server.
-   `docker/http-server-desktop/`: Contains the `Dockerfile` for the `dev-server` binary, which is the desktop version of the server.

## Services

The `docker-compose.yml` file at the root of the repository defines the following services:

-   `gkg`: A service for the `gkg` command-line interface.
-   `http-server-deployed`: The main web server for the deployed application. It exposes a public-facing API and serves the frontend.
-   `http-server-desktop`: The desktop version of the server, typically used for local development and interaction.

## Building and Running the Application

To build and run the entire stack, you can use Docker Compose from the root of the repository.

### Prerequisites

-   Docker and Docker Compose must be installed on your system.
-   A `jwt.secret` file must be present in the root directory for the `http-server-deployed` service to start. You can create one with a dummy value for local development by running the following command in the root of the repository:
    ```bash
    echo "dummy-secret-for-development" > jwt.secret
    ```

### Build the Images

To build all the Docker images for the services, run the following command from the root of the repository:

```bash
docker compose build
```

### Run the Services

To start all the services in detached mode, run:

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

(e.g., `docker compose logs -f http-server-deployed`)

### Stop the Services

To stop all running services, run:

```bash
docker compose down
```