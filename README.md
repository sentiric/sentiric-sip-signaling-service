# Sentiric SIP Signaling Service

**Description:** This is the core service for managing SIP call signaling (call setup, management, and termination) within the Sentiric platform. It acts as the orchestrator for call flows by interacting with other specialized microservices.

**Core Responsibilities:**
*   **SIP Message Processing:** Listening for, parsing, validating, and building all types of SIP messages (INVITE, ACK, BYE, CANCEL, REGISTER, OPTIONS, etc.).
*   **Call Session Lifecycle Management:** Managing the state and progression of active call sessions from initiation to termination.
*   **SIP Transport Layer:** Handling incoming and outgoing SIP traffic over UDP, TCP, and TLS (SIPS).
*   **Call Flow Orchestration:** Coordinating various steps of a call by making API calls to:
    *   `sentiric-user-service` for user authentication and registration management.
    *   `sentiric-dialplan-service` for dynamic call routing decisions.
    *   `sentiric-media-service` for allocating and managing real-time media (RTP/SRTP) sessions.
*   **CDR Event Publishing:** Asynchronously publishing critical call lifecycle events (e.g., call initiated, answered, terminated) to the `sentiric-cdr-service` for detailed record-keeping.
*   **TLS Support:** Managing TLS certificates and secure connections for SIPS.

**Technologies:**
*   Node.js (for its event-driven architecture and existing codebase foundations)
*   UDP/TCP/TLS Sockets
*   Internal API Client Modules (for communication with other Sentiric services)

**API Interactions (As a Client of other Sentiric Services):**
*   **`sentiric-user-service`**: For user authentication, querying user registration status, and updating contact information.
*   **`sentiric-dialplan-service`**: For obtaining call routing decisions based on dialed numbers and caller context.
*   **`sentiric-media-service`**: For requesting RTP/SRTP session creation, proxying media streams, and triggering media actions like playing announcements.
*   **`sentiric-cdr-service`**: Publishes call events to this service, typically via a message queue (e.g., Kafka, RabbitMQ) for asynchronous processing.

**Local Development:**
1.  **Clone this repository:**
    ```bash
    git clone https://github.com/sentiric/sentiric-sip-signaling-service.git
    ```
2.  **Navigate into the directory:**
    ```bash
    cd sentiric-sip-signaling-service
    ```
3.  **Install dependencies:**
    ```bash
    npm install
    ```
4.  **Configure Environment Variables:** Create a `.env` file in the root directory by copying `.env.example`. This file will contain configurations such as SIP listening IPs/ports, public IP, and crucial URLs for dependent services (e.g., `USER_SERVICE_URL`, `DIALPLAN_SERVICE_URL`, `MEDIA_SERVICE_URL`, `MESSAGE_QUEUE_URL`).
5.  **Start the service:**
    ```bash
    npm start
    # or for development with hot-reloads:
    npm run dev
    ```

**Configuration:**
Refer to the `config/` directory (if applicable) and the `.env.example` file for detailed configurable options. Ensure all necessary API endpoints for other services are correctly set up.

**Deployment:**
This service is designed for containerized deployment (e.g., Docker, Kubernetes). Refer to the `sentiric-infrastructure` repository for comprehensive deployment configurations, including Dockerfiles and Kubernetes manifests.

**Contributing:**
We welcome contributions! Please refer to the [Sentiric Governance](https://github.com/sentiric/sentiric-governance) repository for detailed coding standards, contribution guidelines, and the overall project vision.

**License:**
This project is licensed under the [License](LICENSE).
