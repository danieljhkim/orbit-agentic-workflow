1. In JobListArgs::execute, after fetching jobs, fetch the most recent run for each job from job_history.
2. Add LAST_RUN and LAST_RUN_AT columns to the table output.
3. For --json output, add 'last_run_state' and 'last_run_at' fields to job_to_json.
4. For --ops output, add 'last_run_state' to job_to_signal_json (useful for automated health checks).
5. Optimize: use a single store query to fetch latest runs for all jobs rather than N+1 queries.