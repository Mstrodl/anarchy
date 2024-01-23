FROM docker.io/rustlang/rust@sha256:67a56556d39ca60aa3cea4a5be0dac1bad6eada19f9a6f0096ab7aaf76751e14 as rust
# nightly-bookworm

RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | bash

COPY . /app/

WORKDIR /app/anarchy_web

RUN wasm-pack build --profiling

FROM docker.io/node:21-alpine3.17 as node

ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable

COPY --from=rust /app /app

WORKDIR /app/anarchy_web/www

RUN --mount=type=cache,id=pnpm,target=/pnpm/store pnpm install --frozen-lockfile

RUN NODE_ENV=production pnpm run build

FROM docker.io/nginxinc/nginx-unprivileged as serve
WORKDIR /app
COPY --from=node /app/anarchy_web/www/dist /usr/share/nginx/html
