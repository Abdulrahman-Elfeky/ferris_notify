-- Add migration script here
ALTER TABLE idempotency AlTER COLUMN response_status_code DROP NOT NULL;
ALTER TABLE idempotency AlTER COLUMN response_body DROP NOT NULL;
ALTER TABLE idempotency AlTER COLUMN response_headers DROP NOT NULL;
