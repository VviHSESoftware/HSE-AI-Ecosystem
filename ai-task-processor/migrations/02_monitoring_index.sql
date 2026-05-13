CREATE INDEX IF NOT EXISTS idx_ai_tasks_monitoring
    ON ai_tasks (status, task_type, mode);