use std::time::Duration;

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    configuration::Settings, domain::NewSubscriber, email_client::EmailClient,
    startup::get_connection_pool,
};

type PgTransaction = Transaction<'static, Postgres>;

pub async fn run_worker_until_stop(config: Settings) -> anyhow::Result<()> {
    let email_client = config.email_client.client();

    let pool = get_connection_pool(&config.database);

    worker_loop(pool, email_client).await
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

async fn worker_loop(pool: PgPool, email_client: EmailClient) -> anyhow::Result<()> {
    loop {
        match try_execute_task(&pool, &email_client).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(ExecutionOutcome::TaskCompleted) => {}
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> anyhow::Result<ExecutionOutcome> {
    let (mut transaction, issue_id, email, name) = match dequeue_task(pool).await? {
        Some(t) => t,
        None => return Ok(ExecutionOutcome::EmptyQueue),
    };
    let issue = get_issue(&mut transaction, issue_id).await?;

    match NewSubscriber::parse(email.clone(), name) {
        Ok(subscriber) => {
            if let Err(e) = email_client
                .send_email(
                    &subscriber.email,
                    &subscriber.name,
                    &issue.title,
                    &issue.html_content,
                )
                .await
            {
                tracing::error!(error.message = %e,"Failed to deliver issue to a confirmed subscriber. \
                    Skipping.");
            }
        }
        Err(e) => {
            tracing::error!(error.message = %e,
                "Skipping a confirmed subscriber.\
                their stored contact details are invalid."
            );
        }
    }
    delete_task(transaction, issue_id, &email).await?;

    Ok(ExecutionOutcome::TaskCompleted)
}

async fn dequeue_task(
    pool: &PgPool,
) -> sqlx::Result<Option<(PgTransaction, Uuid, String, String)>> {
    let mut transaction = pool.begin().await?;
    let r = sqlx::query!(
        "
    SELECT newsletter_id, subscriber_email, name 
    FROM issue_delivery_queue
    FOR UPDATE
    SKIP LOCKED
    LIMIT 1
        "
    )
    .fetch_optional(transaction.as_mut())
    .await?;

    Ok(r.map(|r| (transaction, r.newsletter_id, r.subscriber_email, r.name)))
}

async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue 
        WHERE
            newsletter_id = $1 AND
            subscriber_email = $2 
        "#,
        issue_id,
        email
    )
    .execute(transaction.as_mut())
    .await?;
    transaction.commit().await?;
    Ok(())
}

struct NewsletterIssue {
    title: String,
    html_content: String,
}

async fn get_issue(
    transaction: &mut PgTransaction,
    issue_id: Uuid,
) -> sqlx::Result<NewsletterIssue> {
    sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, html_content
        FROM newsletter_issues
        WHERE newsletter_id = $1
        "#,
        issue_id
    )
    .fetch_one(transaction.as_mut())
    .await
}
