#!/bin/sh
set -e

# Initialize PostgreSQL if data directory is empty
if [ ! -s "$PGDATA/PG_VERSION" ]; then
    mkdir -p "$PGDATA"
    chmod 700 "$PGDATA"
    initdb -D "$PGDATA"
    
    # Start PostgreSQL
    pg_ctl -D "$PGDATA" -o "-c listen_addresses='localhost'" -w start
    
    # Create user and database
    psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<-EOSQL
        CREATE USER $POSTGRES_USER WITH PASSWORD '$POSTGRES_PASSWORD';
        CREATE DATABASE $POSTGRES_DB;
        GRANT ALL PRIVILEGES ON DATABASE $POSTGRES_DB TO $POSTGRES_USER;
EOSQL
else
    # Just start PostgreSQL if data directory exists
    pg_ctl -D "$PGDATA" -o "-c listen_addresses='localhost'" -w start
fi

# Function to execute SQL and capture output
execute_sql() {
    QUERY_FILE=$1
    OUTPUT_FILE=$2
    ERROR_FILE=$3
    
    /usr/bin/time -v psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f "$QUERY_FILE" \
        > "$OUTPUT_FILE" 2> "$ERROR_FILE"
}

# Import tables from table.sql
psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -f /sqlizer/table.sql

# Execute user's query and correct query
execute_sql "/sqlizer/query.sql" "/sqlizer/user_output.sql" "/sqlizer/error_output.sql"
execute_sql "/sqlizer/correct_query.sql" "/sqlizer/expected_output.sql" "/sqlizer/error_correct.sql"

# Capture execution metadata
/usr/bin/time -v true > /sqlizer/metadata.json

# Stop PostgreSQL
pg_ctl -D "$PGDATA" -m fast -w stop

exit 0