-- Add migration script here
ALTER TABLE issue_delivery_queue ADD COLUMN name TEXT NOT NULL ;
