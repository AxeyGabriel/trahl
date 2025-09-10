```mermaid
sequenceDiagram
    participant Master
    participant Worker

    Worker->>Master: Hello {params}
    Master->>Worker: HelloAck
    Note over Master: Master discovered worker

    Master->>Worker: ConfigUpdate
    Note over Master: Master updated worker configurations
    Note over Master: Master sent lua scripts to worker

    Master->>Worker: Ping
    Worker->>Master: Pong
    Note over Master,Worker: Periodic keepalive

    Master->>Worker: Job
    Worker->>Master: FileTransferReq {SRC file}
    Master->>Worker: FileChunk
    Master->>Worker: FileChunk
    Worker->>Master: FileTransferOk
    Worker->>Master: JobAck
    Note over Worker: Job is spawned in worker

    Worker->>Master: JobStatus
    Worker->>Master: JobStatus
    Note over Worker: Worker keeps sending job progress & streams stdout

    Worker->>Master: JobDone
    Master->>Worker: FileTransferReq {OUT file}
    Worker->>Master: FileChunk
    Worker->>Master: FileChunk
    Master->>Worker: FileTransferOk
    Note over Worker: Worker informs master the job has completed
