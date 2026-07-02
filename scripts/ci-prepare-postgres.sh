#!/usr/bin/env bash
set -Eeuo pipefail

: "${DATABASE_ADMIN_URL:?DATABASE_ADMIN_URL is required}"
: "${DATABASE_URL:?DATABASE_URL is required}"

echo "Waiting for PostgreSQL..."
for attempt in $(seq 1 30); do
    if pg_isready \
        --dbname="$DATABASE_ADMIN_URL" \
        --quiet
    then
        break
    fi

    if [[ "$attempt" -eq 30 ]]; then
        echo "PostgreSQL did not become ready." >&2
        exit 1
    fi

    sleep 1
done

echo "Creating or refreshing the CI runtime role..."
psql "$DATABASE_ADMIN_URL" \
    -v ON_ERROR_STOP=1 <<'SQL'
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_roles
        WHERE rolname = 'kaspa_pulse_app'
    ) THEN
        CREATE ROLE kaspa_pulse_app
            LOGIN
            PASSWORD 'TEST_PASSWORD';
    ELSE
        ALTER ROLE kaspa_pulse_app
            WITH LOGIN
            PASSWORD 'TEST_PASSWORD';
    END IF;
END
$$;
SQL

echo "Applying migrations in lexical order..."
while IFS= read -r migration; do
    echo "Applying migration: ${migration}"
    psql "$DATABASE_ADMIN_URL" \
        -v ON_ERROR_STOP=1 \
        -f "$migration"
done < <(
    find migrations \
        -maxdepth 1 \
        -type f \
        -name '*.sql' \
        -print |
    sort
)

echo "Verifying the runtime role can access the delivery queue..."
psql "$DATABASE_URL" \
    -v ON_ERROR_STOP=1 \
    -Atqc "
        SELECT
            to_regclass('public.telegram_delivery_queue') IS NOT NULL
            AND has_table_privilege(
                current_user,
                'public.telegram_delivery_queue',
                'SELECT,UPDATE'
            );
    " |
grep -qx 't'

echo "CI PostgreSQL schema is ready."
