## This file is from tests/docker/dockerfiles/debian-hexagon-cross.docker of the qemu repository.
## This file is licensed under GPL-2.0-or-later.
##
## This file has modifications.
##
## Modifications:
## - Lennon Anderson - 2025-08-20

# Containerfile to build QEMU's hexagon tcg tests.
#
# Steps:
# 1. Grab Linaro hexagon toolchain
#   - I think this could also use quic's if they're not the same
#   - https://github.com/quic/toolchain_for_hexagon
# 2. Shallow clone QEMU repo and grab tests
# 3. Build tests via toolchain
#
# TODO add the mutiarch tcg tests

FROM docker.io/library/debian:11-slim AS base

ARG jobs=4

# Duplicate deb line as deb-src
RUN cat /etc/apt/sources.list | sed "s/^deb\ /deb-src /" >> /etc/apt/sources.list
# Pull toolchain deps
RUN apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt install -yy eatmydata && \
    DEBIAN_FRONTEND=noninteractive eatmydata \
# Install common build utilities
    apt-get install -y --no-install-recommends \
        wget \
        xz-utils \
	unzip \
        ca-certificates

ENV TOOLCHAIN_INSTALL   /opt
ENV TOOLCHAIN_VERSION  "6.5.0.0"
ENV TOOLCHAIN_TOOLSVER "19.0.07"
ENV TOOLCHAIN_BASENAME "Hexagon_SDK_Linux.zip"
ENV TOOLCHAIN_URL https://softwarecenter.qualcomm.com/api/download/software/sdks/Hexagon_SDK/Linux/Debian/${TOOLCHAIN_VERSION}/Hexagon_SDK_Linux.zip

# https://serverfault.com/questions/735882/unzip-from-stdin-to-stdout-funzip-python
RUN wget "$TOOLCHAIN_URL" -O /tmp/${TOOLCHAIN_BASENAME}
RUN unzip /tmp/${TOOLCHAIN_BASENAME} \
      -d "$TOOLCHAIN_INSTALL" \
      -x "*.qik" \
      "Hexagon_SDK/${TOOLCHAIN_VERSION}/tools/HEXAGON_Tools/${TOOLCHAIN_TOOLSVER}" && \
    rm /tmp/${TOOLCHAIN_BASENAME}
	

ENV TOOLCHAIN_BIN "${TOOLCHAIN_INSTALL}/Hexagon_SDK/${TOOLCHAIN_VERSION}/tools/HEXAGON_Tools/${TOOLCHAIN_TOOLSVER}/Tools/bin"
ENV PATH $PATH:$TOOLCHAIN_BIN
ENV MAKE /usr/bin/make
ENV CC ${TOOLCHAIN_BIN}/hexagon-clang

FROM base AS build
# fetch/build deps
RUN DEBIAN_FRONTEND=noninteractive eatmydata \
    apt-get install -y --no-install-recommends \
        make \
	build-essential \
        git \
        # why need this
        libxml2 \
	cmake \
	ninja-build
RUN mkdir /src
WORKDIR /src

# single commit shallow clone QEMU repo
#RUN mkdir -p qemu && cd qemu && \
#    git init && \
#    git remote add origin https://gitlab.com/qemu-project/qemu.git && \
#    # August 13th, 2025
#    git fetch --depth 1 origin 5836af0783213b9355a6bbf85d9e6bc4c9c9363f && \
#    git checkout FETCH_HEAD

# copy and build tests
#RUN mkdir -p tests/
#WORKDIR tests/
#RUN mkdir -p src/ && \
#    cp ../qemu/tests/tcg/hexagon/* src/
#COPY data/Makefile .
#RUN mkdir -p build/ && make -j $jobs all

# copy and build qemu hexagon testing
WORKDIR /src
RUN git clone https://github.com/qualcomm/qemu-hexagon-testing
WORKDIR /src/qemu-hexagon-testing
RUN cmake -S standalone_systests -B build-systests \
  -G Ninja \
  -DCMAKE_TOOLCHAIN_FILE=${PWD}/cmake/hexagon-standalone.cmake \
  -DHEXAGON_SDK_ROOT=${TOOLCHAIN_INSTALL}/Hexagon_SDK/${TOOLCHAIN_VERSION} \
  -DHEXAGON_ARCH=v71 && \
  cmake --build build-systests

# test artifacts are in /testdata/bin
FROM build AS release
RUN mkdir -p /testdata/bin/qemu-tests
RUN mkdir -p /testdata/bin/qemu-hexagon-testing
WORKDIR /testdata
#COPY --from=build /src/tests/build/* bin/qemu-tests
COPY --from=build /src/qemu-hexagon-testing/build-systests/bin/* bin/qemu-hexagon-testing

WORKDIR /src

# container will exit (then rm) after 100 seconds
ENTRYPOINT [ "sh", "-c", "sleep 100" ]
