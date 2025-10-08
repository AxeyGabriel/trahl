<a id="readme-top"></a>

<!-- LOGO -->
<br />
<p align="center">
    <a href="https://github.com/AxeyGabriel/trahl">
        <img src="https://raw.githubusercontent.com/AxeyGabriel/trahl/refs/heads/master/.github/assets/logo.png" width="250">
    </a>
</p>

[![CI][ci-shield]][ci-url]

# Trahl
A distributed media transcoding system scriptable via Lua

<!-- SUMMARY -->
## Summary
1. [About the Project](#about-the-project)
2. [Installation](#installation)
   - [Docker](#docker)
   - [From source](#from-source)
3. [Configuration](#configuration)
4. [Lua API](#lua-runtime)
   - [Utils package](#utils-package)
   - [Integrations package](#integrations-package)
5. [Web interface](#web-interface)
6. [Example](#example)
7. [Contributing](#contributing)
8. [License](#license)

<!-- ABOUT -->
## About The Project
Trahl is a distributed transcoding system written in Rust and fully customizable via Lua. It is designed to handle media processing at scale while giving users full control over workflows and job pipelines.

<!-- INSTALLATION -->
## Installation
Trahl is supported in FreeBSD, Linux and Windows
<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Docker
You can run **Trahl** using Docker and `docker-compose`. The image is available on GitHub Container Registry: `ghcr.io/axeygabriel/trahl:latest`.

### Steps

1. **Create a `docker-compose.yml`** file:

```yaml
version: "3.9"

services:
  trahl:
    image: ghcr.io/axeygabriel/trahl:latest
    container_name: trahl
    environment:
      CONFIG_FILE: "/config/trahl.yaml" # Path inside container to your config
      MODE: "master"                    # "master" or "worker"
    volumes:
      - ./config:/config                # Mount configuration
      - ./db:/db                        # Mount path for trahl internal files
    restart: unless-stopped
```
2. Create a [configuration file](#configuration)
3. Start Trahl:
```bash
$ docker-compose up -d
```
4. Check logs:
```bash
$ docker-compose logs -f
```
5. Done!

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### From source

* Linux
```bash
$ cargo build --release
$ sudo ./install.sh
```

* FreeBSD

* Windows
<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- Configuration -->
## Configuration
todo</br>
Master and Worker can share the same configuration file if it's convenient</br>

```toml
# Trahl configuration file
# Filename: config.toml

[[jobs]]
name = "Transcode Movies"                       # Library name
enabled = true                                  # Library enable switch
source_path = "/media/source/movies"            # Source path to discover new jobs
destination_path = "/media/destination/movies"  # Location where to processed file
lua_script = "/configs/scripts/movie.lua"       # Script to be executed
[jobs.variables]                                # Add variables to lua context, accesible in _trahl.vars table
EXCLUDECODEC = "h265"                           # Example variable. Regex matching works

[master]
web_ui_port = 8080                              # Web interface port
orch_bind_addr = "127.0.0.1:8245"               # Orchestration bind address

[worker]
master_addr = "127.0.0.1:8245"                  # Master orchestration ip:port
identifier = "test-worker"                      # Worker name
cache_dir = "/tmp/trahl-cache"                  # Workdir / Transcode cache (write intensive)
mapped = true                                   # Filesystem mapped node
fs_remaps = [                                   # If master and worker diverges in path,
    { master = "/", worker = "/" }              # you can control remapping here
]

[log]
level = "debug"
file = "/dev/stdout"
```

You can test and see your parsed configuration by running
```bash trahl
$ trahl -c config.toml -t
```
<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- Lua runtime -->
## Lua runtime
todo
<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Utils package
todo
<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Integrations Package
todo
<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- Web interface -->
## Web Interface
todo
<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- Example -->
## Example
todo
<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- Contributing -->
## Contributing
Any contributions you make are greatly appreciated

If you have a suggestion that would make this better, please fork the repo and create a pull request or simply open an issue with the tag "enhancement".

Don't forget to give the project a star!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Top contributors:

<a href="https://github.com/AxeyGabriel/trahl/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=AxeyGabriel/trahl" alt="contrib.rocks image" />
</a>

See all contributors [here](https://github.com/AxeyGabriel/trahl/graphs/contributors)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- LICENSE -->
## License

Distributed under the BSD 2-Clause license. See `LICENSE` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

[ci-shield]: https://github.com/AxeyGabriel/trahl/actions/workflows/ci.yml/badge.svg?style=for-the-badge&branch=master
[ci-url]: https://github.com/AxeyGabriel/trahl/actions/workflows/ci.yml
