ARG WORKER_BASE_IMAGE
FROM ${WORKER_BASE_IMAGE}
# Base must be the compatible sweettest execution userspace, including Node.
# The packing script requires an immutable digest and tests Node availability.
COPY --chmod=755 compute-executor ark /usr/local/bin/
COPY *.mjs /opt/compute/
RUN node --version && /usr/local/bin/compute-executor --help
USER 0:0
ENTRYPOINT ["node", "/opt/compute/worker.mjs"]
