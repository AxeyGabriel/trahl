```mermaid
flowchart TD
    n1["Start"] --> n2["Parse args"]
    n2 --> n3["Parse conf"]
    n3 --> n4["Init logging"]
    n4 --> n5["Setup sighandler"]
    n5 --> n6["Start Master"] & n7["Start Worker"]
    n6 --> n8["SIGINT or SIGTERM"]
    n7 --> n8
    n8 --> n9["Stop"]

    n1@{ shape: start}
    n9@{ shape: stop}
