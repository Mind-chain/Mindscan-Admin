FROM node:18-alpine as base

# Check https://github.com/nodejs/docker-node/tree/b4117f9333da4138b03a546ec926ef50a31506c3#nodealpine to understand why libc6-compat might be needed.

RUN apk add --no-cache libc6-compat bash git curl

FROM base as deps

COPY package.json yarn.lock ./

ENV NODE_ENV="production"
RUN yarn install --frozen-lockfile --production

FROM deps as run

# Create app directory
WORKDIR /app

COPY --from=deps /node_modules ./node_modules

COPY . .
ENV NODE_ENV="production"
RUN yarn build:prod && rm -rf src


CMD yarn start:prod
