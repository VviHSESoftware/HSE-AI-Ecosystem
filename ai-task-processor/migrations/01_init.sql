CREATE TABLE IF NOT EXISTS ai_tasks (
    task_id UUID PRIMARY KEY,
    task_type VARCHAR(255) NOT NULL,
    mode VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    payload JSONB NOT NULL,
    task_result JSONB,
    error TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_ai_tasks_payload ON ai_tasks USING GIN (payload);