# trahl
Distributed media transcoding

todo
Master:
    Create a Job discovery thread which can discover files via
        - inotify
        - manual discovery
        - scheduled discovery
        * Jobs needs to be deduplicated, storing source filename in a database for future testing
        Those "Jobs" need to be sent via a channel to task_manager


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
