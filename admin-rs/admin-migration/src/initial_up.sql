CREATE TYPE "submission_status" AS ENUM (
  'in_process',
  'waiting_for_update',
  'approved',
  'rejected'
);

CREATE TABLE "users" (
  "id" BIGSERIAL PRIMARY KEY,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "email" varchar UNIQUE NOT NULL,
  "password" varchar NOT NULL,
  "is_superuser" bool NOT NULL DEFAULT false
);

CREATE TABLE "users_chains" (
  "user_id" bigserial NOT NULL,
  "chain_id" bigint NOT NULL,
  PRIMARY KEY ("user_id", "chain_id")
);

CREATE TABLE "submissions" (
  "id" BIGSERIAL PRIMARY KEY,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "updated_at" timestamp NOT NULL DEFAULT (now()),
  "chain_id" bigint NOT NULL,
  "token_address" varchar NOT NULL,
  "status" submission_status NOT NULL DEFAULT 'in_process',
  "blockscout_user_email" varchar NOT NULL,
  "requester_name" varchar NOT NULL,
  "requester_email" varchar NOT NULL,
  "project_name" varchar,
  "project_website" varchar NOT NULL,
  "project_email" varchar NOT NULL,
  "icon_url" varchar NOT NULL,
  "project_description" varchar NOT NULL,
  "project_sector" varchar,
  "comment" varchar,
  "docs" varchar,
  "github" varchar,
  "telegram" varchar,
  "linkedin" varchar,
  "discord" varchar,
  "slack" varchar,
  "twitter" varchar,
  "open_sea" varchar,
  "facebook" varchar,
  "medium" varchar,
  "reddit" varchar,
  "support" varchar,
  "coin_market_cap_ticker" varchar,
  "coin_gecko_ticker" varchar,
  "defi_llama_ticker" varchar
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON submissions
FOR EACH ROW
EXECUTE PROCEDURE trigger_set_timestamp();

CREATE TABLE "waiting_for_update_submissions" (
  "id" BIGSERIAL PRIMARY KEY,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "submission_id" bigserial,
  "admin_comments" varchar NOT NULL,
  "addressed" bool NOT NULL DEFAULT false
);

CREATE TABLE "rejected_submissions" (
  "id" BIGSERIAL PRIMARY KEY,
  "created_at" timestamp NOT NULL DEFAULT (now()),
  "submission_id" bigserial NOT NULL,
  "reason" varchar NOT NULL
);

COMMENT ON TABLE "submissions" IS 'There should be at most one submission 
for given token address that is `in_process` or `waiting_for_update`';

COMMENT ON TABLE "waiting_for_update_submissions" IS 'At any given moment only one non-addressed waiting
should occur for any submission.';

ALTER TABLE "users_chains" ADD FOREIGN KEY ("user_id") REFERENCES "users" ("id");

ALTER TABLE "waiting_for_update_submissions" ADD FOREIGN KEY ("submission_id") REFERENCES "submissions" ("id");

ALTER TABLE "rejected_submissions" ADD FOREIGN KEY ("submission_id") REFERENCES "submissions" ("id");
