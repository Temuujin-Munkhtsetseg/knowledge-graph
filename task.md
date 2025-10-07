# Task: Refactor Project Dockerization

Hello Jules,

Please perform a comprehensive refactoring of the Docker setup for the `knowledge-graph` project. The goal is to create a clean, organized, and scalable containerization strategy where each main crate application has its own dedicated image and the entire system can be managed via a central Docker Compose file.

### 1. Create New Docker Directory Structure

-   Create a new directory named `docker` at the root of the repository.
-   Inside `docker`, create subdirectories for each of the following applications:
    -   `gkg`
    -   `http-server-deployed`
    -   `http-server-desktop` (the `dev-server` binary)

### 2. Individual Dockerfiles for Each Application

-   For each application subdirectory (e.g., `docker/gkg/`), create a dedicated `Dockerfile`.
-   Each `Dockerfile` must use an optimized, multi-stage build process to compile only its specific binary from the source code.
-   Ensure the final runtime stage is slim, containing only the compiled binary and necessary runtime dependencies.

### 3. Update Application Code for Web Deployment

-   Thoroughly inspect the source code for all three applications (`gkg`, `http-server-deployed`, `http-server-desktop`).
-   Identify any network listeners that are hardcoded to bind to `127.0.0.1`.
-   Modify the source code to make them bind to `0.0.0.0` to ensure they are accessible from outside their containers.

### 4. Create Root Docker Compose File

-   Create a new `docker-compose.yml` file at the root of the repository.
-   This file should define services for all three applications.
-   Each service definition should:
    -   Use `build.context: .` and `build.dockerfile:` to point to the correct `Dockerfile` inside the `docker` directory.
    -   Map appropriate ports.
    -   Configure necessary volumes for data persistence and source code mounting.
    -   Set any required environment variables.

### 5. Build Context Optimization

-   Create or update the root `.dockerignore` file to exclude common development artifacts like `.git`, `target/`, `node_modules/`, and any local secrets or environment files.

### 6. Documentation

-   Create a `README.md` file inside the new `docker` directory.
-   This file should briefly explain the new directory structure, describe what each service is for, and provide clear instructions on how to build and run the entire stack.

### 7. Verification and Testing

-   **Crucial Step:** After creating all the `Dockerfile`s and the `docker-compose.yml`, you must verify that the images build successfully.
-   Run `docker compose build` from the root directory.
-   **You must ensure that all services build without any errors before committing the new files.** If a build fails, debug the issue in the corresponding `Dockerfile` or source code and retry until all builds are successful.

Please proceed with this task, ensuring the final result is a clean, well-organized, and fully functional multi-service Docker environment that has been build-tested.
