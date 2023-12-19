FROM docker.io/rust:1.73-buster as rust

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
