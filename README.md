# trahl
Distributed media transcoding

todo
Master:
    Job discovery
        inotify + manual/scheduled discovery
        jobs dedup with heuristic hash
    Job orchestration
    Web view
    Job file server
Worker:
    Job receiver
    ffmpeg/handbrake spawner task - progress channels

Lua:
    file utils ffi - fsremap if worker

Master/worker communication
    Master listens on socket
    Worker connects to master and send HELLO
    Master discovers worker and send HELLOACK
    Master push job to worker
    Worker receive job and start lua script
        Lua scripts takes care of the processing pipeline
        If lua script ends with ok, job done
        else job error
    Worker sends job progress periodically to master
    Worker sends job done or job error to master

    Worker keeps sending KEEPALIVE packets to master
