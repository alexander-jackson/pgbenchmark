# `pgbenchmark`

`pgbenchmark` is a tool for benchmarking PostgreSQL queries against a live
database. It allows you to provide:

1. The current query
2. Commands to make schema changes, such as create indexes
3. The proposed query (which may be the same as the current query)
4. Commands to rollback the changes made in (2)
5. Bind parameters for the query
6. Connection details for the database

It will connect to the database and run some warmups as well as live runs for
each of the parameter groups, recording the planning and execution times. After
that, it will apply the schema changes and run the proposed query in the same
manner before rolling back the schema changes.

It uses this information to compare whether the suggested query (alongside any
schema changes) will have a performance improvement.

## Usage

```bash
pgbenchmark \
    --current query.sql \
    --up up.sql \
    --proposed query.sql \
    --down down.sql \
    --parameters parameters.txt \
    --connection-details connection-details.txt
```

`connection-details.txt` is expected to contain a single line with a PostgreSQL
connection string.
