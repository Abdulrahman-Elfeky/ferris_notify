-- Add migration script here
CREATE TABLE newsletter_issues(
  newsletter_id uuid PRIMARY KEY,
  title TEXT NOT NULL,
  html_content TEXT NOT NULL,
  published_at timestamptz
);
