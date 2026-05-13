use crate::schemas::{ResultItem, SubmissionTextResponse, VerdictResponse};
use sea_query::{Expr, Iden, PostgresQueryBuilder, Query};
use sqlx::{PgPool, Row};
use std::path::Path;
use sea_query_binder::SqlxBinder;
use uuid::Uuid;

#[derive(Iden)]
#[iden(rename = "ai_tasks")]
pub enum AiTasks {
    Table,
    TaskId,
    TaskType,
    Mode,
    Status,
    Payload,
    TaskResult,
    Error,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Clone)]
pub struct AutocheckRepository {
    pool: PgPool,
}

pub struct QueueStat {
    pub task_type: String,
    pub state: String,
    pub count: i64,
}

pub struct TaskStatusStat {
    pub task_type: String,
    pub mode: String,
    pub status: String,
    pub count: i64,
}

impl AutocheckRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error>> {
        let migrator = sqlx::migrate::Migrator::new(Path::new("./migrations")).await?;
        migrator.run(&self.pool).await?;
        Ok(())
    }

    pub async fn get_queue_depth(&self) -> Result<Vec<QueueStat>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                task_identifier as task_type,
                CASE
                    WHEN locked_at IS NOT NULL THEN 'processing'
                    WHEN attempts >= max_attempts THEN 'failed'
                    WHEN run_at > now() AND attempts > 0 THEN 'retrying'
                    WHEN run_at > now() THEN 'scheduled'
                    ELSE 'pending'
                END as state,
                count(*) as count
            FROM graphile_worker.jobs
            GROUP BY task_type, state
            "#
        )
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| QueueStat {
            task_type: r.get("task_type"),
            state: r.get("state"),
            count: r.get("count"),
        }).collect())
    }

    pub async fn get_active_tasks_stats(&self) -> Result<Vec<TaskStatusStat>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                task_type,
                mode,
                status,
                count(*) as count
            FROM ai_tasks
            WHERE status IN ('pending', 'processing')
            GROUP BY task_type, mode, status
            "#
        )
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| TaskStatusStat {
            task_type: r.get("task_type"),
            mode: r.get("mode"),
            status: r.get("status"),
            count: r.get("count"),
        }).collect())
    }

    pub async fn create_task(&self, task_id: Uuid, task_type: &str, mode: &str, payload: serde_json::Value) -> Result<(), sqlx::Error> {
        let (sql, values) = Query::insert()
            .into_table(AiTasks::Table)
            .columns([AiTasks::TaskId, AiTasks::TaskType, AiTasks::Mode, AiTasks::Status, AiTasks::Payload])
            .values_panic([task_id.into(), task_type.into(), mode.into(), "pending".into(), payload.into()])
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_with(&sql, values).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_task_processing(&self, task_id: Uuid) -> Result<(), sqlx::Error> {
        let (sql, values) = Query::update()
            .table(AiTasks::Table)
            .values([
                (AiTasks::Status, "processing".into()),
                (AiTasks::UpdatedAt, Expr::cust("NOW()").into()),
            ])
            .and_where(Expr::col(AiTasks::TaskId).eq(task_id))
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_with(&sql, values).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_task_completed(&self, task_id: Uuid, result: serde_json::Value) -> Result<(), sqlx::Error> {
        let (sql, values) = Query::update()
            .table(AiTasks::Table)
            .values([
                (AiTasks::Status, "completed".into()),
                (AiTasks::TaskResult, result.into()),
                (AiTasks::UpdatedAt, Expr::cust("NOW()").into()),
            ])
            .and_where(Expr::col(AiTasks::TaskId).eq(task_id))
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_with(&sql, values).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_task_error(&self, task_id: Uuid, error: &str) -> Result<(), sqlx::Error> {
        let (sql, values) = Query::update()
            .table(AiTasks::Table)
            .values([
                (AiTasks::Status, "error".into()),
                (AiTasks::Error, error.into()),
                (AiTasks::UpdatedAt, Expr::cust("NOW()").into()),
            ])
            .and_where(Expr::col(AiTasks::TaskId).eq(task_id))
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_with(&sql, values).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_results_by_ids(&self, ids: &[Uuid]) -> Result<Vec<ResultItem>, sqlx::Error> {
        let (sql, values) = Query::select()
            .columns([AiTasks::TaskId, AiTasks::Status, AiTasks::TaskType, AiTasks::TaskResult, AiTasks::Error])
            .from(AiTasks::Table)
            .and_where(Expr::col(AiTasks::TaskId).is_in(ids.iter().copied()))
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_as_with::<_, ResultItem, _>(&sql, values).fetch_all(&self.pool).await
    }
    
    pub async fn get_students_by_task(&self, task_name: &str, only_failed: bool) -> Result<Vec<String>, sqlx::Error> {
        let mut query = Query::select();
        query.distinct()
            .expr(Expr::cust("payload->>'email'"))
            .from(AiTasks::Table)
            .and_where(Expr::cust("payload->>'task_name'").eq(task_name));

        if only_failed {
            query.and_where(Expr::col(AiTasks::Status).eq("error"));
        }

        let (sql, values) = (&query).build_sqlx(PostgresQueryBuilder);

        let rows: Vec<Option<String>> = sqlx::query_scalar_with(&sql, values)
            .fetch_all(&self.pool).await?;

        Ok(rows.into_iter().flatten().collect())
    }

    pub async fn get_latest_submission(&self, task_name: &str, email: &str) -> Result<Option<SubmissionTextResponse>, sqlx::Error> {
        let (sql, values) = Query::select()
            .column(AiTasks::Payload)
            .from(AiTasks::Table)
            .and_where(Expr::cust("payload->>'task_name'").eq(task_name))
            .and_where(Expr::cust("payload->>'email'").eq(email))
            .order_by(AiTasks::CreatedAt, sea_query::Order::Desc)
            .limit(1)
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_as_with::<_, SubmissionTextResponse, _>(&sql, values).fetch_optional(&self.pool).await
    }

    pub async fn get_latest_verdict(&self, task_name: &str, email: &str) -> Result<Option<VerdictResponse>, sqlx::Error> {
        let (sql, values) = Query::select()
            .columns([AiTasks::Status, AiTasks::TaskResult, AiTasks::Error, AiTasks::Mode, AiTasks::CreatedAt])
            .from(AiTasks::Table)
            .and_where(Expr::cust("payload->>'task_name'").eq(task_name))
            .and_where(Expr::cust("payload->>'email'").eq(email))
            .order_by(AiTasks::CreatedAt, sea_query::Order::Desc)
            .limit(1)
            .build_sqlx(PostgresQueryBuilder);

        sqlx::query_as_with::<_, VerdictResponse, _>(&sql, values).fetch_optional(&self.pool).await
    }
}