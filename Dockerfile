ARG ENABLE_NV="no"
ARG ENABLE_QSV="no"
ARG ENABLE_DOLBYVISION="no"

FROM rust:latest as builder
WORKDIR /usr/src/trahl
COPY . .
RUN cargo build --release

FROM docker.io/rockylinux/rockylinux:10 as handbrake
WORKDIR /usr/src/handbrake
RUN dnf -y update && \
    dnf -y install --nogpgcheck https://download1.rpmfusion.org/free/el/rpmfusion-free-release-$(rpm -E %rhel).noarch.rpm && \
    dnf -y install --nogpgcheck https://download1.rpmfusion.org/nonfree/el/rpmfusion-nonfree-release-$(rpm -E %rhel).noarch.rpm && \
	dnf -y config-manager --set-enabled crb && \
	dnf -y install autoconf automake bzip2 cmake diffutils dnf-plugins-core fribidi-devel gcc-c++ git libtool libxml2-devel m4 make numactl-devel patch pkg-config python3 tar xz-devel && \
	dnf -y install jansson-devel lame-devel libogg-devel libsamplerate-devel libtheora-devel libvorbis-devel libvpx-devel meson nasm ninja-build opus-devel speex-devel turbojpeg-devel && \
	dnf -y install libass-devel x264-devel libva-devel libdrm-devel openssl-devel git curl && \
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	
RUN if [ "$ENABLE_NV" = "yes" ]; then \
		dnf -y install epel-release && \
		dnf -y config-manager --add-repo http://developer.download.nvidia.com/compute/cuda/repos/rhel10/x86_64/cuda-rhel10.repo && \
		dnf -y install cuda-toolkit; \
	fi

RUN if [ "$ENABLE_DOLBYVISION" = "yes" ]; then \
		source $HOME/.cargo/env && \
		cargo install cargo-c; \
	fi

RUN source $HOME/.cargo/env && \
	git clone --branch 1.9.2 --depth 1 https://github.com/HandBrake/HandBrake.git . && \
	./configure --launch-jobs=$(nproc) --launch --disable-gtk \
		$( [ "$ENABLE_NVDEC" = "yes" ] && echo " --enable-nvdec --enable-nvenc" ) \
		$( [ "$ENABLE_NVDEC" = "no" ] && echo " --disable-nvdec --disable-nvenc" ) \
		$( [ "$ENABLE_QSV" = "yes" ] && echo " --enable-qsv" ) \
		$( [ "$ENABLE_DOLBYVISION" = "yes" ] && echo " --enable-libdovi" ) \
		--enable-fdk-aac --enable-vce

FROM docker.io/rockylinux/rockylinux:10-minimal
WORKDIR /app
RUN microdnf -y install dnf && \
    dnf -y update && \
	dnf -y install epel-release && \
    dnf -y install --nogpgcheck https://download1.rpmfusion.org/free/el/rpmfusion-free-release-$(rpm -E %rhel).noarch.rpm && \
    dnf -y install --nogpgcheck https://download1.rpmfusion.org/nonfree/el/rpmfusion-nonfree-release-$(rpm -E %rhel).noarch.rpm && \
	dnf -y config-manager --set-enabled crb && \
    dnf -y install ffmpeg ca-certificates && \
	dnf -y install fribidi-devel libtool libxml2-devel numactl-devel python3 xz-devel && \
	dnf -y install jansson-devel lame-devel libogg-devel libsamplerate-devel libtheora-devel libvorbis-devel libvpx-devel meson opus-devel speex-devel turbojpeg-devel && \
	dnf -y install libass-devel x264-devel libva-devel libdrm-devel openssl-devel && \
	dnf -y clean all

# TODO install drivers

COPY --from=builder /usr/src/trahl/target/release/trahl /usr/local/bin/trahl
COPY --from=handbrake /usr/src/handbrake /usr/src/handbrake
RUN make --directory=/usr/src/handbrake/build install && \
	rm -rf /usr/src/handbrake

COPY docker/entrypoint.sh /entrypoint.sh

RUN chmod +x /usr/local/bin/trahl /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD []
