FROM debian:stable-slim

# Install required packages
RUN apt-get update && \
    apt-get install -y dpkg-dev && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /build

WORKDIR /build

# Add a script to handle the build process
COPY build-deb.sh /usr/local/bin/build-deb
RUN chmod +x /usr/local/bin/build-deb

# Set the entrypoint to our build script
ENTRYPOINT ["/usr/local/bin/build-deb"]