# 🛡️ Sentiric SIP Signaling Service

**Description:** This is the core edge service for managing SIP call signaling (setup, management, and termination) within the Sentiric platform. Built with **Rust** for high performance, memory safety, and low-level network control, it acts as the primary orchestrator for the synchronous phase of call flows by interacting with other specialized microservices.

**Core Responsibilities:**
*   **High-Performance SIP Processing:** Listens for, parses, and validates SIP messages over UDP. It is specifically hardened to handle real-world telecommunication challenges, such as correctly managing `Via` and `Record-Route` headers from upstream proxies.
*   **Synchronous Call Flow Orchestration:** Rapidly coordinates the initial steps of a call by making **gRPC** calls to:
    *   `sentiric-user-service` for user authentication.
    *   `sentiric-dialplan-service` for dynamic call routing decisions.
    *   `sentiric-media-service` for allocating real-time media (RTP) sessions.
*   **Asynchronous Event Publishing:** After successfully establishing a call, it decouples the long-running AI dialogue logic by publishing a `call.started` event to a **RabbitMQ** message queue, making the platform resilient and scalable.
*   **Security:** Acts as the first line of defense for the platform's real-time communication infrastructure.

**Technology Stack:**
*   **Language:** Rust
*   **Async Runtime:** Tokio
*   **Inter-Service Communication:**
    *   **gRPC (with Tonic):** For fast, type-safe, synchronous commands.
    *   **AMQP (with Lapin):** For resilient, asynchronous eventing via RabbitMQ.
*   **Containerization:** Docker (Multi-stage builds for minimal, secure images).

**API Interactions (Client Of):**
*   **`sentiric-user-service` (gRPC):** For user authentication.
*   **`sentiric-dialplan-service` (gRPC):** For obtaining call routing decisions.
*   **`sentiric-media-service` (gRPC):** For requesting RTP session creation.
*   **`RabbitMQ` (AMQP):** Publishes `call.started` events to decouple the agent/AI workflow.

## Getting Started

### Prerequisites
- Docker and Docker Compose
- Git
- All Sentiric repositories cloned into a single workspace directory.

### Local Development & Platform Setup
This service is not designed to run standalone. It is an integral part of the Sentiric platform and must be run via the central orchestrator in the `sentiric-infrastructure` repository.

1.  **Clone all repositories:**
    ```bash
    # In your workspace directory
    git clone https://github.com/sentiric/sentiric-infrastructure.git
    git clone https://github.com/sentiric/sentiric-core-interfaces.git
    git clone https://github.com/sentiric/sentiric-sip-signaling-service.git
    # ... clone other required services
    ```

2.  **Initialize Submodules:** This service depends on `sentiric-core-interfaces` using a Git submodule.
    ```bash
    cd sentiric-sip-signaling-service
    git submodule update --init --recursive
    cd .. 
    ```

3.  **Configure Environment:**
    ```bash
    cd sentiric-infrastructure
    cp .env.local.example .env
    # Open .env and set PUBLIC_IP and other variables
    ```

4.  **Run the platform:** The central Docker Compose file will automatically build and run this service.
    ```bash
    # From the sentiric-infrastructure directory
    docker compose up --build -d
    ```

5.  **View Logs:**
    ```bash
    docker compose logs -f sip-signaling
    ```

## Configuration

All configuration is managed via environment variables passed from the `sentiric-infrastructure` repository's `.env` file. See the `.env.local.example` file in that repository for a complete list.

## Deployment

This service is designed for containerized deployment. The multi-stage `Dockerfile` ensures a small and secure production image. The CI/CD pipeline in `.github/workflows/docker-ci.yml` automatically builds and pushes the image to the GitHub Container Registry (`ghcr.io`).

## Contributing

We welcome contributions! Please refer to the [Sentiric Governance](https://github.com/sentiric/sentiric-governance) repository for detailed coding standards, contribution guidelines, and the overall project vision.

## License

This project is licensed under the [License](LICENSE).
