use sqlx::{Executor, PgPool};
use tracing::info;

pub async fn init_db(pool: &PgPool) {
    pool.execute(r#"
        CREATE TABLE IF NOT EXISTS modules (
            id SERIAL PRIMARY KEY,
            parent_id INT REFERENCES modules(id) ON DELETE CASCADE,
            "name" TEXT NOT NULL,
            "type" TEXT NOT NULL, -- 'Text', 'Video', 'File', 'System', 'Base'
            url TEXT,
            external_id TEXT,
            external_type TEXT,
            path TEXT -- /id/id/id
        );

        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            email TEXT UNIQUE NOT NULL
        );

        CREATE TABLE IF NOT EXISTS module_access (
            module_id INT REFERENCES modules(id) ON DELETE CASCADE,
            user_id INT REFERENCES users(id) ON DELETE CASCADE,
            PRIMARY KEY (module_id, user_id)
        );

        CREATE TABLE IF NOT EXISTS module_text (
            module_id INT PRIMARY KEY REFERENCES modules(id) ON DELETE CASCADE,
            content TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS module_video (
            module_id INT PRIMARY KEY REFERENCES modules(id) ON DELETE CASCADE,
            transcription TEXT NOT NULL,
            total_time DOUBLE PRECISION NOT NULL
        );

        CREATE TABLE IF NOT EXISTS video_chunks (
            id SERIAL PRIMARY KEY,
            module_id INT REFERENCES modules(id) ON DELETE CASCADE,
            start_time DOUBLE PRECISION NOT NULL,
            end_time DOUBLE PRECISION NOT NULL,
            content TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS module_file (
            module_id INT PRIMARY KEY REFERENCES modules(id) ON DELETE CASCADE,
            content TEXT NOT NULL,
            filename TEXT NOT NULL,
            page_count INT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS module_file_pages (
            id SERIAL PRIMARY KEY,
            module_id INT REFERENCES modules(id) ON DELETE CASCADE,
            page_number INT NOT NULL,
            content TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS module_system (
            module_id INT PRIMARY KEY REFERENCES modules(id) ON DELETE CASCADE,
            token_hash TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS hash_cache (
            hash TEXT PRIMARY KEY,
            "data" BYTEA NOT NULL
        );
    "#).await.expect("Failed to initialize database schema");
    info!("Database schema initialized successfully");
}