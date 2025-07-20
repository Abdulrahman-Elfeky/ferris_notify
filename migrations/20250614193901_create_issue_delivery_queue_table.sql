-- Add migration script here
CREATE TABLE issue_delivery_queue(
  newsletter_id uuid NOT NULL 
  REFERENCES newsletter_issues(newsletter_id),
  subscriber_email TEXT NOT NULL,
  PRIMARY KEY(newsletter_id, subscriber_email)
)
