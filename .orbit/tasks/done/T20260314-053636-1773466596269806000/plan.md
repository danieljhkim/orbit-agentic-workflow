1. Locate ID generation — likely orbit-store next_id() helper or similar.
2. Change default generated ID to: T<YYYYMMDD>-<HHMMSS> (no nanosecond suffix).
3. On insert, if that ID already exists, retry with -2, -3, etc. until a free slot is found.
4. Apply the same change to job-run ID generation (jrun- prefix).
5. Update any tests asserting on ID format.
6. Existing tasks/runs with old long-form IDs remain readable — no migration needed (IDs are just filenames).