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

## Output

`pgbenchmark` will provide a summary of the executions at the end, which should
indicate whether the performance has improved or regressed:

```bash
== Results ==
+-------------+----------+-----------+----------+----------+-----------+----------+
| UUID        | Cur exec | Prop exec | Δ exec   | Cur plan | Prop plan | Δ plan   |
+-------------+----------+-----------+----------+----------+-----------+----------+
| 00ca...139f | 0.02ms   | 0.02ms    | -0.54% ↓ | 0.07ms   | 0.06ms    | -0.76% ↓ |
+-------------+----------+-----------+----------+----------+-----------+----------+
| 00f4...d6eb | 0.02ms   | 0.02ms    | -0.52% ↓ | 0.06ms   | 0.07ms    | +0.97% ↑ |
+-------------+----------+-----------+----------+----------+-----------+----------+
| 00f5...63c0 | 0.03ms   | 0.02ms    | -7.58% ↓ | 0.07ms   | 0.06ms    | -2.28% ↓ |
+-------------+----------+-----------+----------+----------+-----------+----------+
| 013a...4436 | 0.02ms   | 0.02ms    | +1.53% ↑ | 0.06ms   | 0.07ms    | +0.97% ↑ |
+-------------+----------+-----------+----------+----------+-----------+----------+
| 0096...d6bb | 0.02ms   | 0.03ms    | +6.00% ↑ | 0.07ms   | 0.07ms    | +0.19% ↑ |
+-------------+----------+-----------+----------+----------+-----------+----------+
Execution time:  avg -0.22% ↓   min -7.58% ↓   max +6.00% ↑   improved 3/5
Planning time:   avg -0.18% ↓   min -2.28% ↓   max +0.97% ↑   improved 2/5
```
