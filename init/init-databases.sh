#!/bin/bash
set -e

create_db() {
    local db_name=$1
    echo "  Creating database: $db_name"
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
        CREATE DATABASE $db_name;
        GRANT ALL PRIVILEGES ON DATABASE $db_name TO $POSTGRES_USER;
EOSQL
}

echo "Starting multiple database initialization..."

create_db "task_processing_db"
create_db "knowledge_base_db"

echo "All databases created successfully!"