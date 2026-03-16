# Orbit

Orbit is a local-first workflow engine for agent-driven software delivery. 

It is currently work-in-progress - so I highly advise against using it on production environment.

Although incomplete, I am already finding it very useful. I would conservatively say it has made me 30% more productive. 

You can check out .orbit to get a glimpse of how it works (don't commit these on your projects though). 

- `.orbit/jobs/jobs`: contains atomic work that can be performed by agents.
- `.orbit/jobs/jobs`: contains executable workflows, chained together by one or more activities.
- `.orbit/jobs/runs`: contains execution run artifacts (execution audits).
- `.orbit/skills`: contains orbit-related skills to enable agentic paired-programming capabilities.
- `.orbit/taks`: contains task artifacts, akin to Jira tickets.

---

## Obit Agent Dynamics

Claude is solid but codex does the heavy lifting here. 

Codex has much more generous usage limits - highly recommend it.

Thus in this project, code executions are assigned to codex; and claude for planning and reviewing.

## Issues

If you find any issues, feel free to create an issue or raise a PR. 